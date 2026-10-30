use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// How app bookings are paid; chosen by admins in Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum PaymentMode {
    /// Pay right after booking; unpaid bookings are canceled after [`super::PAY_NOW_MINUTES`].
    PayNow,
    /// Pay within the admin-set time, or the booking is canceled.
    PayLater,
}

impl PaymentMode {
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentMode::PayNow => "pay_now",
            PaymentMode::PayLater => "pay_later",
        }
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PaymentSettings {
    pub mode: PaymentMode,
    /// Time allowed to pay in `pay_later` mode.
    pub pay_later_minutes: i32,
    pub updated_at: DateTime<Utc>,
}

/// The booking fields payments need.
#[derive(Debug, Clone, FromRow)]
pub struct PayableBooking {
    pub id: Uuid,
    /// The player who made the booking.
    pub player_id: Uuid,
    pub bookeo_booking_number: String,
    pub product_name: String,
    pub start_time: DateTime<Utc>,
    pub status: String,
    pub price_amount: Option<String>,
    pub price_currency: Option<String>,
    /// `not_required`, `unpaid`, `partially_paid` or `paid`.
    pub payment_status: String,
    /// Paid so far, e.g. "120.00".
    pub amount_paid: String,
    pub payment_mode: Option<PaymentMode>,
    pub payment_due_at: Option<DateTime<Utc>>,
}

/// One SkipCash payment (its payment page). `open` until it's paid or we stop using it.
#[derive(Debug, Clone, FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub booking_id: Uuid,
    /// Who is paying: the booking's player or one of its other players.
    pub payer_id: Option<Uuid>,
    pub merchant_txn_id: String,
    /// SkipCash's payment id.
    pub provider_payment_id: Option<String>,
    /// The card processor's reference for a paid payment (reconciliation).
    pub visa_id: Option<String>,
    pub pay_url: Option<String>,
    pub amount: String,
    pub currency: String,
    pub last_failure: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
}
