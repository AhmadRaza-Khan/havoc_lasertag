use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::model::{PayableBooking, Payment, PaymentMode, PaymentSettings};

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

pub async fn settings(db: &PgPool) -> sqlx::Result<PaymentSettings> {
    sqlx::query_as("SELECT mode, pay_later_minutes, updated_at FROM payment_settings")
        .fetch_one(db)
        .await
}

pub async fn save_settings(
    db: &PgPool,
    mode: PaymentMode,
    pay_later_minutes: i32,
) -> sqlx::Result<PaymentSettings> {
    sqlx::query_as(
        "UPDATE payment_settings SET mode = $1, pay_later_minutes = $2, updated_at = now()
         RETURNING mode, pay_later_minutes, updated_at",
    )
    .bind(mode)
    .bind(pay_later_minutes)
    .fetch_one(db)
    .await
}

// ---------------------------------------------------------------------------
// Bookings
// ---------------------------------------------------------------------------

/// Serializes payment work on one booking (app requests and the worker) until `tx` ends.
pub async fn lock_booking(
    tx: &mut Transaction<'_, Postgres>,
    booking_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(booking_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// The booking; with `player_id`, only if that player made it or is one of its other players.
pub async fn booking(
    db: &PgPool,
    id: Uuid,
    player_id: Option<Uuid>,
) -> sqlx::Result<Option<PayableBooking>> {
    sqlx::query_as(
        "SELECT id, player_id, bookeo_booking_number, product_name, start_time,
                status, price_amount, price_currency, payment_status, amount_paid::text AS amount_paid,
                payment_mode, payment_due_at
         FROM bookings
         WHERE id = $1 AND ($2::uuid IS NULL OR player_id = $2 OR $2 = ANY(other_players))",
    )
    .bind(id)
    .bind(player_id)
    .fetch_optional(db)
    .await
}

pub async fn require_payment(
    db: &PgPool,
    id: Uuid,
    mode: PaymentMode,
    due_at: DateTime<Utc>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE bookings SET payment_status = 'unpaid', payment_mode = $2, payment_due_at = $3,
             updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(mode)
    .bind(due_at)
    .execute(db)
    .await?;
    Ok(())
}

/// Unpaid, not canceled, past their deadline.
pub async fn overdue_bookings(db: &PgPool, limit: i64) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        "SELECT id FROM bookings
         WHERE payment_status = 'unpaid' AND status <> 'canceled' AND payment_due_at < now()
         ORDER BY payment_due_at LIMIT $1",
    )
    .bind(limit)
    .fetch_all(db)
    .await
}

// ---------------------------------------------------------------------------
// Payments
// ---------------------------------------------------------------------------

const COLUMNS: &str = "id, booking_id, payer_id, merchant_txn_id, provider_payment_id, visa_id, pay_url, amount, \
    currency, status, last_failure, paid_at, bookeo_payment_id, created_at";

/// Reserves a payment row (and our transaction id) before calling SkipCash.
pub async fn insert(
    db: &PgPool,
    booking_id: Uuid,
    payer_id: Uuid,
    amount: &str,
    currency: &str,
) -> sqlx::Result<Payment> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO payments (booking_id, payer_id, amount, currency) VALUES ($1, $2, $3, $4)
         RETURNING {COLUMNS}"
    )))
    .bind(booking_id)
    .bind(payer_id)
    .bind(amount)
    .bind(currency)
    .fetch_one(db)
    .await
}

pub async fn set_link(
    db: &PgPool,
    id: Uuid,
    provider_payment_id: &str,
    url: &str,
) -> sqlx::Result<Payment> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE payments SET provider_payment_id = $2, pay_url = $3, updated_at = now()
         WHERE id = $1 RETURNING {COLUMNS}"
    )))
    .bind(id)
    .bind(provider_payment_id)
    .bind(url)
    .fetch_one(db)
    .await
}

/// The booking's most recent link that can still be paid.
pub async fn open_for_booking(db: &PgPool, booking_id: Uuid) -> sqlx::Result<Vec<Payment>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM payments
         WHERE booking_id = $1 AND status = 'open'
         ORDER BY created_at DESC"
    )))
    .bind(booking_id)
    .fetch_all(db)
    .await
}

/// The booking's latest paid payment.
pub async fn last_paid(db: &PgPool, booking_id: Uuid) -> sqlx::Result<Option<Payment>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM payments WHERE booking_id = $1 AND status = 'paid'
         ORDER BY paid_at DESC LIMIT 1"
    )))
    .bind(booking_id)
    .fetch_optional(db)
    .await
}

/// Open payments SkipCash should be asked about, checked longest ago first.
pub async fn open_links(db: &PgPool, limit: i64) -> sqlx::Result<Vec<Payment>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM payments
         WHERE status = 'open' AND provider_payment_id IS NOT NULL
         ORDER BY updated_at LIMIT $1"
    )))
    .bind(limit)
    .fetch_all(db)
    .await
}

/// Payments we stopped using in the last 75 minutes. SkipCash can't disable a payment page;
/// it cancels unfinished ones after an hour, so until then a late payment is still possible.
pub async fn recently_closed(db: &PgPool, limit: i64) -> sqlx::Result<Vec<Payment>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM payments
         WHERE status = 'closed' AND provider_payment_id IS NOT NULL
           AND created_at > now() - interval '75 minutes'
         ORDER BY updated_at LIMIT $1"
    )))
    .bind(limit)
    .fetch_all(db)
    .await
}

/// A payment by SkipCash's id (webhooks).
pub async fn by_provider_id(
    db: &PgPool,
    provider_payment_id: &str,
) -> sqlx::Result<Option<Payment>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM payments WHERE provider_payment_id = $1"
    )))
    .bind(provider_payment_id)
    .fetch_optional(db)
    .await
}

/// A payment by our transaction id (webhooks for retries SkipCash files under a new id).
pub async fn by_merchant_txn_id(
    db: &PgPool,
    merchant_txn_id: &str,
) -> sqlx::Result<Option<Payment>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM payments WHERE merchant_txn_id = $1"
    )))
    .bind(merchant_txn_id)
    .fetch_optional(db)
    .await
}

/// Open links whose booking was canceled (by staff, Bookeo, or for non-payment).
pub async fn open_links_of_canceled_bookings(
    db: &PgPool,
    limit: i64,
) -> sqlx::Result<Vec<Payment>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {} FROM payments p JOIN bookings b ON b.id = p.booking_id
         WHERE p.status = 'open' AND b.status = 'canceled'
         LIMIT $1",
        prefixed("p")
    )))
    .bind(limit)
    .fetch_all(db)
    .await
}

fn prefixed(alias: &str) -> String {
    COLUMNS
        .split(", ")
        .map(|c| format!("{alias}.{c}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Records that SkipCash was asked (so the worker rotates through payments) and the latest failure.
pub async fn touch(db: &PgPool, id: Uuid, last_failure: Option<&str>) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE payments SET last_failure = COALESCE($2, last_failure), updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(last_failure)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn close(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE payments SET status = 'closed', updated_at = now() WHERE id = $1 AND status = 'open'")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// Marks the payment paid and updates its booking's total paid and status (`paid` once the
/// price is covered, else `partially_paid`). `false` if it was already paid (nothing changed).
/// Works on a closed payment too: money received late still counts.
pub async fn mark_paid(
    db: &PgPool,
    payment: &Payment,
    visa_id: Option<&str>,
) -> sqlx::Result<bool> {
    let mut tx = db.begin().await?;
    let updated = sqlx::query(
        "UPDATE payments SET status = 'paid', paid_at = now(), visa_id = COALESCE($2, visa_id), updated_at = now()
         WHERE id = $1 AND status <> 'paid'",
    )
    .bind(payment.id)
    .bind(visa_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if updated == 0 {
        return Ok(false);
    }
    sqlx::query(
        "WITH paid AS (
             SELECT COALESCE(SUM(amount::numeric), 0) AS total
             FROM payments WHERE booking_id = $1 AND status = 'paid'
         )
         UPDATE bookings b SET
             amount_paid = paid.total,
             payment_status = CASE
                 WHEN b.price_amount IS NOT NULL AND paid.total >= b.price_amount::numeric THEN 'paid'
                 ELSE 'partially_paid' END,
             updated_at = now()
         FROM paid WHERE b.id = $1",
    )
    .bind(payment.booking_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

/// Claims a paid payment for recording in Bookeo. Claims expire so a crash mid-way is retried.
pub async fn claim_bookeo_sync(db: &PgPool, id: Uuid) -> sqlx::Result<Option<Payment>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE payments SET bookeo_sync_at = now()
         WHERE id = $1 AND status = 'paid' AND bookeo_payment_id IS NULL
           AND (bookeo_sync_at IS NULL OR bookeo_sync_at < now() - interval '2 minutes')
         RETURNING {COLUMNS}"
    )))
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn set_bookeo_payment_id(db: &PgPool, id: Uuid, bookeo_id: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE payments SET bookeo_payment_id = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(bookeo_id)
        .execute(db)
        .await?;
    Ok(())
}

/// Paid payments not yet recorded in Bookeo.
pub async fn unsynced(db: &PgPool, limit: i64) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        "SELECT id FROM payments
         WHERE status = 'paid' AND bookeo_payment_id IS NULL
           AND (bookeo_sync_at IS NULL OR bookeo_sync_at < now() - interval '2 minutes')
         ORDER BY paid_at LIMIT $1",
    )
    .bind(limit)
    .fetch_all(db)
    .await
}
