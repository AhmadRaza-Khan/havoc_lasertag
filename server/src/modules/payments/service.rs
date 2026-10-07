use chrono::{DateTime, Duration, FixedOffset, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    PAY_NOW_MINUTES, emails,
    model::{PayableBooking, Payment, PaymentMode, PaymentSettings},
    repository,
};
use crate::{
    error::{AppError, AppResult},
    integrations::{
        bookeo::{Money, NewPayment},
        qnb::{LinkStatus, PayLinkRequest, QnbClient, QnbError},
    },
    modules::{bookings, players::repository as players},
    state::AppState,
};

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    #[serde(flatten)]
    pub settings: PaymentSettings,
    /// Fixed time allowed to pay in `pay_now` mode.
    pub pay_now_minutes: i64,
    /// False when the server has no QNB credentials: app bookings have nothing to pay.
    pub online_payment_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct SettingsInput {
    pub mode: PaymentMode,
    pub pay_later_minutes: i32,
}

pub const MIN_PAY_LATER_MINUTES: i32 = 15;
pub const MAX_PAY_LATER_MINUTES: i32 = 30 * 24 * 60;

pub async fn settings(state: &AppState) -> AppResult<SettingsResponse> {
    Ok(settings_response(
        state,
        repository::settings(&state.db).await?,
    ))
}

pub async fn save_settings(state: &AppState, input: SettingsInput) -> AppResult<SettingsResponse> {
    if !(MIN_PAY_LATER_MINUTES..=MAX_PAY_LATER_MINUTES).contains(&input.pay_later_minutes) {
        return Err(AppError::BadRequest(
            "Time to pay must be between 15 minutes and 30 days".into(),
        ));
    }
    let saved = repository::save_settings(&state.db, input.mode, input.pay_later_minutes).await?;
    tracing::info!(
        mode = saved.mode.as_str(),
        minutes = saved.pay_later_minutes,
        "payment settings saved"
    );
    Ok(settings_response(state, saved))
}

fn settings_response(state: &AppState, settings: PaymentSettings) -> SettingsResponse {
    SettingsResponse {
        settings,
        pay_now_minutes: PAY_NOW_MINUTES,
        online_payment_enabled: state.qnb.is_some(),
    }
}

// ---------------------------------------------------------------------------
// Booking lifecycle
// ---------------------------------------------------------------------------

/// After a booking is made in the app: decides whether and by when it must be paid.
/// In `pay_later` mode a link is created up front and emailed to the player.
pub async fn on_booking_created(state: &AppState, booking_id: Uuid) -> AppResult<()> {
    let Some(qnb) = &state.qnb else { return Ok(()) };
    let Some(booking) = repository::booking(&state.db, booking_id, None).await? else {
        return Ok(());
    };
    if payable_amount(&booking).is_none() {
        return Ok(());
    }

    let settings = repository::settings(&state.db).await?;
    let window = match settings.mode {
        PaymentMode::PayNow => Duration::minutes(PAY_NOW_MINUTES),
        PaymentMode::PayLater => Duration::minutes(settings.pay_later_minutes.into()),
    };
    // Never leave a booking unpaid past the start of its game.
    let due_at = (Utc::now() + window).min(booking.start_time);
    repository::require_payment(&state.db, booking_id, settings.mode, due_at).await?;
    tracing::info!(%booking_id, mode = settings.mode.as_str(), %due_at, "booking awaits payment");

    if settings.mode == PaymentMode::PayLater {
        let booking = PayableBooking {
            payment_status: "unpaid".into(),
            payment_mode: Some(settings.mode),
            payment_due_at: Some(due_at),
            ..booking
        };
        // Without a link the email still asks them to pay in the app.
        let link = new_link(state, qnb, &booking, false)
            .await
            .inspect_err(|e| tracing::error!(%booking_id, "could not create pay link: {e}"))
            .ok()
            .and_then(|p| p.pay_url);
        notify(state, &booking, |to, name| {
            emails::payment_requested(state, to, name, &booking, link.as_deref())
        })
        .await;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Mobile app
// ---------------------------------------------------------------------------

/// What the app needs to show and pay a booking.
#[derive(Debug, Serialize)]
pub struct PaymentView {
    pub booking_id: Uuid,
    /// `unpaid`, `paid` or `not_required`.
    pub status: String,
    pub mode: Option<PaymentMode>,
    pub amount: Option<String>,
    pub currency: Option<String>,
    /// Unpaid bookings are canceled after this. Business timezone.
    pub due_at: Option<DateTime<FixedOffset>>,
    /// QNB's payment page, to open in a WebView. Only while unpaid.
    pub pay_url: Option<String>,
    /// Why the latest attempt failed (e.g. card declined), if it did.
    pub last_failure: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
}

/// Returns a payment page for the booking: the open one if it's still good, else a new one.
pub async fn start(
    state: &AppState,
    booking_id: Uuid,
    player_id: Uuid,
    arabic: bool,
) -> AppResult<PaymentView> {
    let qnb = state.qnb.as_ref().ok_or_else(payment_off)?;
    let mut lock = state.db.begin().await?;
    repository::lock_booking(&mut lock, booking_id).await?;

    let booking = find(state, booking_id, Some(player_id)).await?;
    match booking.payment_status.as_str() {
        "paid" => return view(state, booking).await,
        "not_required" => {
            return Err(AppError::Conflict(
                "This booking has nothing to pay online".into(),
            ));
        }
        _ => {}
    }
    if booking.status == "canceled" {
        return Err(AppError::Conflict("This booking was canceled".into()));
    }
    if booking.payment_due_at.is_some_and(|due| due <= Utc::now()) {
        return Err(AppError::Conflict(
            "The time to pay for this booking has passed".into(),
        ));
    }
    let (amount, _) = payable_amount(&booking)
        .ok_or_else(|| AppError::Conflict("This booking has nothing to pay online".into()))?;

    // Reuse the newest open link while it's unpaid and for the right amount; close the rest.
    let mut reusable = None;
    for p in repository::open_for_booking(&state.db, booking_id).await? {
        if reusable.is_none() && p.amount == amount && p.pay_url.is_some() {
            match check(state, qnb, &p).await? {
                LinkStatus::Accepted { .. } => {
                    return view(state, refreshed(state, booking_id).await?).await;
                }
                LinkStatus::Pending => {
                    reusable = Some(p);
                    continue;
                }
                // Asking again after a failed attempt: start over with a fresh link.
                LinkStatus::Rejected(_) => {}
            }
        }
        if close_link(state, qnb, &p).await? {
            return view(state, refreshed(state, booking_id).await?).await;
        }
    }
    if reusable.is_none() {
        new_link(state, qnb, &booking, arabic).await?;
    }

    lock.commit().await?;
    view(state, refreshed(state, booking_id).await?).await
}

/// The booking's payment state, confirmed with QNB while unpaid. The app polls this while
/// the payment page is open.
pub async fn status(state: &AppState, booking_id: Uuid, player_id: Uuid) -> AppResult<PaymentView> {
    let booking = find(state, booking_id, Some(player_id)).await?;
    if booking.payment_status == "unpaid"
        && let Some(qnb) = &state.qnb
        && let Some(p) = repository::open_for_booking(&state.db, booking_id)
            .await?
            .into_iter()
            .next()
    {
        check(state, qnb, &p).await?;
        return view(state, refreshed(state, booking_id).await?).await;
    }
    view(state, booking).await
}

async fn view(state: &AppState, booking: PayableBooking) -> AppResult<PaymentView> {
    let tz = state.config.bookeo.timezone;
    let amount = payable_amount(&booking);
    let (pay_url, last_failure) = if booking.payment_status == "unpaid" {
        repository::open_for_booking(&state.db, booking.id)
            .await?
            .into_iter()
            .next()
            .map(|p| (p.pay_url, p.last_failure))
            .unwrap_or_default()
    } else {
        (None, None)
    };
    let paid_at = match booking.payment_status.as_str() {
        "paid" => repository::paid_for_booking(&state.db, booking.id)
            .await?
            .and_then(|p| p.paid_at),
        _ => None,
    };
    Ok(PaymentView {
        booking_id: booking.id,
        status: booking.payment_status,
        mode: booking.payment_mode,
        amount: amount.as_ref().map(|(a, _)| a.clone()),
        currency: amount.map(|(_, c)| c),
        due_at: booking
            .payment_due_at
            .map(|d| d.with_timezone(&tz).fixed_offset()),
        pay_url,
        last_failure,
        paid_at,
    })
}

// ---------------------------------------------------------------------------
// Background work (see `worker`)
// ---------------------------------------------------------------------------

/// Asks QNB about an open link; settles it if paid.
pub async fn poll_link(state: &AppState, payment: &Payment) -> AppResult<()> {
    if let Some(qnb) = &state.qnb {
        check(state, qnb, payment).await?;
    }
    Ok(())
}

/// Closes an open link whose booking was canceled. A last-second payment is still settled.
pub async fn close_for_canceled_booking(state: &AppState, payment: &Payment) -> AppResult<()> {
    let Some(qnb) = &state.qnb else { return Ok(()) };
    let mut lock = state.db.begin().await?;
    repository::lock_booking(&mut lock, payment.booking_id).await?;
    if close_link(state, qnb, payment).await? {
        tracing::error!(
            booking_id = %payment.booking_id,
            ipg_txn_id = ?payment.ipg_txn_id,
            "canceled booking was paid; refund it from the QNB merchant console"
        );
    }
    lock.commit().await?;
    Ok(())
}

/// Cancels a booking whose time to pay has run out, unless it gets paid meanwhile.
pub async fn expire(state: &AppState, booking_id: Uuid) -> AppResult<()> {
    let mut lock = state.db.begin().await?;
    repository::lock_booking(&mut lock, booking_id).await?;
    let booking = find(state, booking_id, None).await?;
    let still_overdue = booking.payment_status == "unpaid"
        && booking.status != "canceled"
        && booking.payment_due_at.is_some_and(|d| d <= Utc::now());
    if still_overdue {
        cancel_unpaid(state, &booking, Unpaid::TimeRanOut).await?;
    }
    lock.commit().await?;
    Ok(())
}

/// The customer closed the payment page without paying. In "pay now" mode the booking is
/// canceled right away (a payment completed at the last second still counts). In "pay later"
/// mode nothing changes: they can pay until the deadline.
pub async fn abandon(
    state: &AppState,
    booking_id: Uuid,
    player_id: Uuid,
) -> AppResult<PaymentView> {
    let mut lock = state.db.begin().await?;
    repository::lock_booking(&mut lock, booking_id).await?;
    let booking = find(state, booking_id, Some(player_id)).await?;
    if booking.payment_status == "unpaid"
        && booking.status != "canceled"
        && booking.payment_mode == Some(PaymentMode::PayNow)
    {
        cancel_unpaid(state, &booking, Unpaid::LeftPaymentPage).await?;
    }
    lock.commit().await?;
    view(state, refreshed(state, booking_id).await?).await
}

/// Why an unpaid booking is canceled.
#[derive(Debug, Clone, Copy)]
pub enum Unpaid {
    TimeRanOut,
    LeftPaymentPage,
}

/// Closes the booking's payment links, then cancels it in Bookeo and emails the customer.
/// If a link turns out to be paid, the booking is kept (and settled) instead.
/// Call with the booking's lock held.
async fn cancel_unpaid(state: &AppState, booking: &PayableBooking, why: Unpaid) -> AppResult<()> {
    // Close the links first so nobody can pay for a booking that's about to be canceled.
    if let Some(qnb) = &state.qnb {
        for p in repository::open_for_booking(&state.db, booking.id).await? {
            if close_link(state, qnb, &p).await? {
                return Ok(());
            }
        }
    }

    bookings::cancel_for_nonpayment(state, booking.id).await?;
    tracing::info!(booking_id = %booking.id, reason = ?why, "unpaid booking canceled");
    notify(state, booking, |to, name| {
        emails::canceled_unpaid(state, to, name, booking, why)
    })
    .await;
    Ok(())
}

/// Records a paid payment in Bookeo (once; safe to call repeatedly and concurrently).
pub async fn record_in_bookeo(state: &AppState, payment_id: Uuid) -> AppResult<()> {
    let Some(p) = repository::claim_bookeo_sync(&state.db, payment_id).await? else {
        return Ok(());
    };
    let booking = find(state, p.booking_id, None).await?;
    let ipg = p.ipg_txn_id.clone().unwrap_or_default();
    let comment = format!("QNB transaction {ipg} ({})", p.merchant_txn_id);

    // A previous try may have reached Bookeo without us saving the result.
    let existing = state
        .bookeo
        .booking_payments(&booking.bookeo_booking_number)
        .await?
        .into_iter()
        .find(|bp| bp.comment.as_deref().is_some_and(|c| c.contains(&comment)));
    let bookeo_id = match existing {
        Some(bp) => bp.id,
        None => {
            state
                .bookeo
                .add_payment(
                    &booking.bookeo_booking_number,
                    &NewPayment {
                        received_time: p
                            .paid_at
                            .unwrap_or_else(Utc::now)
                            .to_rfc3339_opts(SecondsFormat::Secs, true),
                        reason: "Online payment".into(),
                        comment,
                        amount: Money {
                            amount: p.amount.clone(),
                            currency: p.currency.clone(),
                        },
                        payment_method: "creditCard",
                    },
                )
                .await?
                .id
        }
    };
    repository::set_bookeo_payment_id(&state.db, p.id, &bookeo_id).await?;
    tracing::info!(booking_id = %p.booking_id, %bookeo_id, "payment recorded in bookeo");
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn find(state: &AppState, id: Uuid, player_id: Option<Uuid>) -> AppResult<PayableBooking> {
    repository::booking(&state.db, id, player_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Booking not found".into()))
}

async fn refreshed(state: &AppState, id: Uuid) -> AppResult<PayableBooking> {
    find(state, id, None).await
}

fn payment_off() -> AppError {
    AppError::Conflict("Online payment is not available".into())
}

/// Creates a QNB link for the booking's price.
async fn new_link(
    state: &AppState,
    qnb: &QnbClient,
    booking: &PayableBooking,
    arabic: bool,
) -> AppResult<Payment> {
    let (amount, currency) = payable_amount(booking).ok_or_else(payment_off)?;
    let row = repository::insert(&state.db, booking.id, &amount, &currency).await?;
    let tz = state.config.bookeo.timezone;
    let due_date = booking
        .payment_due_at
        .unwrap_or(booking.start_time)
        .with_timezone(&tz)
        .date_naive();

    let link = qnb
        .create_pay_link(&PayLinkRequest {
            merchant_txn_id: &row.merchant_txn_id,
            amount: &amount,
            currency: &currency,
            reference: &format!("Booking {}", booking.bookeo_booking_number),
            due_date,
            arabic,
        })
        .await;
    match link {
        Ok(link) => {
            tracing::info!(booking_id = %booking.id, ipg_txn_id = %link.ipg_txn_id, "pay link created");
            Ok(repository::set_link(&state.db, row.id, &link.ipg_txn_id, &link.url).await?)
        }
        Err(e) => {
            repository::close(&state.db, row.id).await?;
            Err(e.into())
        }
    }
}

/// Asks QNB about a link and acts on the answer.
async fn check(state: &AppState, qnb: &QnbClient, p: &Payment) -> AppResult<LinkStatus> {
    let Some(ipg) = &p.ipg_txn_id else {
        return Ok(LinkStatus::Pending);
    };
    let status = qnb.link_status(ipg).await?;
    match &status {
        LinkStatus::Accepted { amount, currency } => settle(state, p, amount, currency).await?,
        LinkStatus::Rejected(reason) => repository::touch(&state.db, p.id, Some(reason)).await?,
        LinkStatus::Pending => repository::touch(&state.db, p.id, None).await?,
    }
    Ok(status)
}

/// Disables a link at QNB, then checks it one last time. Returns `true` if it turned out paid
/// (and was settled); otherwise it's closed. Errors if QNB can't be reached, so the link is
/// never treated as closed while it might still be payable.
async fn close_link(state: &AppState, qnb: &QnbClient, p: &Payment) -> AppResult<bool> {
    let Some(ipg) = &p.ipg_txn_id else {
        repository::close(&state.db, p.id).await?;
        return Ok(false);
    };
    match qnb.cancel_pay_link(ipg).await {
        Ok(()) => {}
        // e.g. already paid or expired; the inquiry below tells which.
        Err(QnbError::Rejected(e)) => {
            tracing::info!(ipg_txn_id = %ipg, "qnb refused to cancel link: {e}")
        }
        Err(e) => return Err(e.into()),
    }
    if let LinkStatus::Accepted { amount, currency } = qnb.link_status(ipg).await? {
        settle(state, p, &amount, &currency).await?;
        return Ok(true);
    }
    repository::close(&state.db, p.id).await?;
    Ok(false)
}

/// QNB accepted the payment: mark it paid (once), record it in Bookeo, email a receipt.
async fn settle(state: &AppState, p: &Payment, amount: &str, currency: &str) -> AppResult<()> {
    if normalize_amount(amount).as_deref() != Some(p.amount.as_str()) || currency != p.currency {
        tracing::warn!(
            payment_id = %p.id,
            expected = %format!("{} {}", p.amount, p.currency),
            charged = %format!("{amount} {currency}"),
            "qnb charged a different amount"
        );
    }
    let newly_paid = match repository::mark_paid(&state.db, p).await {
        Ok(changed) => changed,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            // Another link of this booking was paid first.
            tracing::error!(
                booking_id = %p.booking_id,
                ipg_txn_id = ?p.ipg_txn_id,
                "booking paid twice; refund this transaction from the QNB merchant console"
            );
            repository::touch(&state.db, p.id, Some("Paid twice: refund needed")).await?;
            repository::close(&state.db, p.id).await?;
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };
    if !newly_paid {
        return Ok(());
    }
    let booking_id = p.booking_id;
    tracing::info!(%booking_id, ipg_txn_id = ?p.ipg_txn_id, "booking paid");
    let booking = find(state, booking_id, None).await?;
    if booking.status == "canceled" {
        tracing::error!(%booking_id, "payment received for a canceled booking; refund needed");
    }
    after_payment(state, &booking, p).await;
    Ok(())
}

/// A booking was paid: record the payment in Bookeo (in the background) and email a receipt.
async fn after_payment(state: &AppState, booking: &PayableBooking, p: &Payment) {
    let state2 = state.clone();
    let payment_id = p.id;
    tokio::spawn(async move {
        if let Err(e) = record_in_bookeo(&state2, payment_id).await {
            // The worker retries.
            tracing::warn!(%payment_id, "could not record payment in bookeo yet: {e}");
        }
    });
    notify(state, booking, |to, name| {
        emails::payment_received(state, to, name, booking, p)
    })
    .await;
}

/// Emails the booking's player, if they have an email address.
async fn notify(
    state: &AppState,
    booking: &PayableBooking,
    build: impl FnOnce(&str, &str) -> crate::integrations::email::Email,
) {
    let player_id = booking.player_id;
    match players::find_by_id(&state.db, player_id).await {
        Ok(Some(player)) => {
            if let Some(email) = &player.email {
                state.mailer.send_later(build(email, &player.full_name));
            }
        }
        Ok(None) => {}
        Err(e) => tracing::error!(%player_id, "could not load player for email: {e}"),
    }
}

/// The amount to charge online, or `None` if there's nothing to pay.
fn payable_amount(b: &PayableBooking) -> Option<(String, String)> {
    let amount = normalize_amount(b.price_amount.as_deref()?)?;
    let currency = b.price_currency.clone().filter(|c| c.len() == 3)?;
    if amount.chars().all(|c| c == '0' || c == '.') {
        return None;
    }
    Some((amount, currency))
}

/// "500" / "500.5" / "500.500" → "500.00" / "500.50" / "500.50". `None` if not a plain
/// non-negative decimal with at most 2 significant decimals.
pub fn normalize_amount(s: &str) -> Option<String> {
    let s = s.trim();
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let digits = |p: &str| p.chars().all(|c| c.is_ascii_digit());
    if int.is_empty() || !digits(int) || !digits(frac) {
        return None;
    }
    let frac = frac.trim_end_matches('0');
    if frac.len() > 2 {
        return None;
    }
    let int = int.trim_start_matches('0');
    Some(format!(
        "{}.{frac:0<2}",
        if int.is_empty() { "0" } else { int }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_amounts() {
        assert_eq!(normalize_amount("500").as_deref(), Some("500.00"));
        assert_eq!(normalize_amount("500.5").as_deref(), Some("500.50"));
        assert_eq!(normalize_amount("0500.500").as_deref(), Some("500.50"));
        assert_eq!(normalize_amount("0").as_deref(), Some("0.00"));
        assert_eq!(normalize_amount("1.005"), None);
        assert_eq!(normalize_amount("-5"), None);
        assert_eq!(normalize_amount("5e3"), None);
        assert_eq!(normalize_amount(""), None);
    }
}
