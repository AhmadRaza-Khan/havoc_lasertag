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
        skipcash::{NewPayment as SkipCashPayment, PaymentState, SkipCashClient, WebhookPayload},
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
    /// False when the server has no SkipCash credentials: app bookings have nothing to pay.
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
        online_payment_enabled: state.skipcash().is_some(),
    }
}

// ---------------------------------------------------------------------------
// Booking lifecycle
// ---------------------------------------------------------------------------

/// After a booking is made in the app: decides whether and by when it must be paid, and
/// emails the player the deadline (with a pay link in `pay_later` mode).
pub async fn on_booking_created(state: &AppState, booking_id: Uuid) -> AppResult<()> {
    let Some(skipcash) = state.skipcash() else {
        return Ok(());
    };
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

    let booking = PayableBooking {
        payment_status: "unpaid".into(),
        payment_mode: Some(settings.mode),
        payment_due_at: Some(due_at),
        ..booking
    };
    // "Pay later": a link up front, so the email can carry it. "Pay now": the app opens the
    // payment page itself; the email just states the deadline.
    let link = match settings.mode {
        PaymentMode::PayLater => new_link(state, &skipcash, &booking)
            .await
            .inspect_err(|e| tracing::error!(%booking_id, "could not create pay link: {e}"))
            .ok()
            .and_then(|p| p.pay_url),
        PaymentMode::PayNow => None,
    };
    notify(state, &booking, |to, name| {
        emails::payment_requested(state, to, name, &booking, link.as_deref())
    })
    .await;
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
    /// SkipCash's payment page, to open in a WebView. Only while unpaid.
    pub pay_url: Option<String>,
    /// Why the latest attempt failed (e.g. card declined), if it did.
    pub last_failure: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
}

/// Returns a payment page for the booking: the open one if it's still good, else a new one.
pub async fn start(state: &AppState, booking_id: Uuid, player_id: Uuid) -> AppResult<PaymentView> {
    let skipcash = state.skipcash().ok_or_else(payment_off)?;
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

    // Reuse the newest open payment while it's still payable and for the right amount; stop
    // using the rest.
    let mut reusable = None;
    for p in repository::open_for_booking(&state.db, booking_id).await? {
        if reusable.is_none() && p.amount == amount && p.pay_url.is_some() {
            match check(state, &skipcash, &p).await? {
                PaymentState::Paid { .. } => {
                    return view(state, refreshed(state, booking_id).await?).await;
                }
                PaymentState::Pending => {
                    reusable = Some(p);
                    continue;
                }
                // Canceled (expired) or failed: `check` closed it; start over with a fresh page.
                _ => continue,
            }
        }
        if retire(state, &skipcash, &p).await? {
            return view(state, refreshed(state, booking_id).await?).await;
        }
    }
    if reusable.is_none() {
        new_link(state, &skipcash, &booking).await?;
    }

    lock.commit().await?;
    view(state, refreshed(state, booking_id).await?).await
}

/// The booking's payment state, confirmed with SkipCash while unpaid. The app polls this
/// while the payment page is open.
pub async fn status(state: &AppState, booking_id: Uuid, player_id: Uuid) -> AppResult<PaymentView> {
    let booking = find(state, booking_id, Some(player_id)).await?;
    if booking.payment_status == "unpaid"
        && let Some(skipcash) = state.skipcash()
        && let Some(p) = repository::open_for_booking(&state.db, booking_id)
            .await?
            .into_iter()
            .next()
    {
        check(state, &skipcash, &p).await?;
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

/// Asks SkipCash about an open payment; settles it if paid.
pub async fn poll_link(state: &AppState, payment: &Payment) -> AppResult<()> {
    if let Some(skipcash) = state.skipcash() {
        check(state, &skipcash, payment).await?;
    }
    Ok(())
}

/// Checks a payment we stopped using: SkipCash keeps its page payable for up to an hour, so a
/// late payment is caught (and settled) here even if no webhook arrives.
pub async fn poll_closed(state: &AppState, payment: &Payment) -> AppResult<()> {
    let (Some(skipcash), Some(id)) = (state.skipcash(), &payment.provider_payment_id) else {
        return Ok(());
    };
    if let PaymentState::Paid {
        amount,
        currency,
        visa_id,
    } = skipcash.payment(id).await?
    {
        settle(state, payment, &amount, &currency, visa_id.as_deref()).await?;
    }
    Ok(())
}

/// Stops using an open payment whose booking was canceled. A last-second payment is still
/// settled (and flagged for a refund).
pub async fn close_for_canceled_booking(state: &AppState, payment: &Payment) -> AppResult<()> {
    let Some(skipcash) = state.skipcash() else {
        return Ok(());
    };
    let mut lock = state.db.begin().await?;
    repository::lock_booking(&mut lock, payment.booking_id).await?;
    retire(state, &skipcash, payment).await?;
    lock.commit().await?;
    Ok(())
}

/// SkipCash's webhook (already verified): settles payments made with the app closed and
/// records failed attempts for the app. Unknown payments are ignored.
pub async fn on_webhook(state: &AppState, w: &WebhookPayload) -> AppResult<()> {
    // A failed attempt is filed under a new SkipCash id with our transaction id (manual §4).
    let payment = match repository::by_provider_id(&state.db, &w.payment_id).await? {
        Some(p) => Some(p),
        None => match w.transaction_id.as_deref().filter(|t| !t.is_empty()) {
            Some(t) => repository::by_merchant_txn_id(&state.db, t).await?,
            None => None,
        },
    };
    let Some(p) = payment else {
        tracing::info!(payment_id = %w.payment_id, "skipcash webhook for an unknown payment; ignored");
        return Ok(());
    };
    let status: i64 = w
        .status_id
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(-1);
    let amount = w.amount.clone().unwrap_or_else(|| p.amount.clone());
    match PaymentState::from_status(status, amount, p.currency.clone(), w.visa_id.clone()) {
        PaymentState::Paid {
            amount,
            currency,
            visa_id,
        } => settle(state, &p, &amount, &currency, visa_id.as_deref()).await?,
        PaymentState::Failed => repository::touch(&state.db, p.id, Some(FAILED_ATTEMPT)).await?,
        PaymentState::Canceled
            if p.provider_payment_id.as_deref() == Some(w.payment_id.as_str()) =>
        {
            repository::close(&state.db, p.id).await?
        }
        PaymentState::Refunded => {
            tracing::warn!(booking_id = %p.booking_id, payment_id = %w.payment_id, "skipcash reports a refund")
        }
        _ => {}
    }
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

/// Checks the booking's open payments one last time and stops using them, then cancels it in
/// Bookeo and emails the customer. If a payment turns out to be paid, the booking is kept (and
/// settled) instead. Call with the booking's lock held.
async fn cancel_unpaid(state: &AppState, booking: &PayableBooking, why: Unpaid) -> AppResult<()> {
    if let Some(skipcash) = state.skipcash() {
        for p in repository::open_for_booking(&state.db, booking.id).await? {
            if retire(state, &skipcash, &p).await? {
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
    let provider_id = p.provider_payment_id.clone().unwrap_or_default();
    let comment = format!("SkipCash payment {provider_id} ({})", p.merchant_txn_id);

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

/// Shown in the app after a card attempt failed (SkipCash doesn't say why).
const FAILED_ATTEMPT: &str = "The card payment didn't go through";

/// Creates a SkipCash payment for the booking's price, in the player's name.
async fn new_link(
    state: &AppState,
    skipcash: &SkipCashClient,
    booking: &PayableBooking,
) -> AppResult<Payment> {
    let (amount, currency) = payable_amount(booking).ok_or_else(payment_off)?;
    let player = players::find_by_id(&state.db, booking.player_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Player not found".into()))?;
    // SkipCash requires an email (sign-up asks for one; accounts staff created may lack it).
    if player.email.as_deref().is_none_or(|e| e.trim().is_empty()) {
        return Err(AppError::Conflict(
            "Online payment needs an email address on your account. Ask our staff to add one, or pay at the venue.".into(),
        ));
    }
    let row = repository::insert(&state.db, booking.id, &amount, &currency).await?;

    // SkipCash wants a first and a last name.
    let mut names = player.full_name.split_whitespace();
    let first = names.next().unwrap_or(&player.playername).to_string();
    let rest = names.collect::<Vec<_>>().join(" ");
    let last = if rest.is_empty() { first.clone() } else { rest };
    let reference = format!("Booking {}", booking.bookeo_booking_number);

    let created = skipcash
        .create_payment(&SkipCashPayment {
            amount: &amount,
            first_name: &first,
            last_name: &last,
            phone: player.phone.as_deref(),
            email: player.email.as_deref(),
            transaction_id: &row.merchant_txn_id,
            custom1: Some(&reference),
        })
        .await;
    match created {
        Ok(c) => {
            tracing::info!(booking_id = %booking.id, skipcash_id = %c.id, "skipcash payment created");
            Ok(repository::set_link(&state.db, row.id, &c.id, &c.pay_url).await?)
        }
        Err(e) => {
            repository::close(&state.db, row.id).await?;
            Err(e.into())
        }
    }
}

/// Asks SkipCash about a payment and acts on the answer: settles it if paid, stops using it if
/// SkipCash canceled it (unfinished for an hour) or it failed.
async fn check(
    state: &AppState,
    skipcash: &SkipCashClient,
    p: &Payment,
) -> AppResult<PaymentState> {
    let Some(id) = &p.provider_payment_id else {
        return Ok(PaymentState::Pending);
    };
    let st = skipcash.payment(id).await?;
    match &st {
        PaymentState::Paid {
            amount,
            currency,
            visa_id,
        } => settle(state, p, amount, currency, visa_id.as_deref()).await?,
        PaymentState::Pending => repository::touch(&state.db, p.id, None).await?,
        PaymentState::Failed => {
            repository::touch(&state.db, p.id, Some(FAILED_ATTEMPT)).await?;
            repository::close(&state.db, p.id).await?;
        }
        PaymentState::Canceled | PaymentState::Refunded => {
            repository::touch(&state.db, p.id, None).await?;
            repository::close(&state.db, p.id).await?;
        }
    }
    Ok(st)
}

/// Stops using a payment: checks it one last time, then closes it. Returns `true` if it turned
/// out paid (and was settled). SkipCash can't disable a payment page, and cancels unfinished
/// ones within an hour; a payment that still arrives (worker or webhook) is settled and logged
/// for a refund if its booking was canceled. Errors if SkipCash can't be reached, so a booking
/// is never canceled while its payment might already be done.
async fn retire(state: &AppState, skipcash: &SkipCashClient, p: &Payment) -> AppResult<bool> {
    if let Some(id) = &p.provider_payment_id
        && let PaymentState::Paid {
            amount,
            currency,
            visa_id,
        } = skipcash.payment(id).await?
    {
        settle(state, p, &amount, &currency, visa_id.as_deref()).await?;
        return Ok(true);
    }
    repository::close(&state.db, p.id).await?;
    Ok(false)
}

/// SkipCash took the payment: mark it paid (once), record it in Bookeo, email a receipt.
async fn settle(
    state: &AppState,
    p: &Payment,
    amount: &str,
    currency: &str,
    visa_id: Option<&str>,
) -> AppResult<()> {
    if normalize_amount(amount).as_deref() != Some(p.amount.as_str()) || currency != p.currency {
        tracing::warn!(
            payment_id = %p.id,
            expected = %format!("{} {}", p.amount, p.currency),
            charged = %format!("{amount} {currency}"),
            "skipcash charged a different amount"
        );
    }
    let newly_paid = match repository::mark_paid(&state.db, p, visa_id).await {
        Ok(changed) => changed,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            // Another payment of this booking was paid first.
            tracing::error!(
                booking_id = %p.booking_id,
                skipcash_id = ?p.provider_payment_id,
                "booking paid twice; refund this payment from the SkipCash merchant portal"
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
    tracing::info!(%booking_id, skipcash_id = ?p.provider_payment_id, "booking paid");
    let booking = find(state, booking_id, None).await?;
    if booking.status == "canceled" {
        tracing::error!(
            %booking_id,
            skipcash_id = ?p.provider_payment_id,
            "payment received for a canceled booking; refund it from the SkipCash merchant portal"
        );
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
