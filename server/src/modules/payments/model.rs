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
    pub player_id: Uuid,
    pub bookeo_booking_number: String,
    pub product_name: String,
    pub start_time: DateTime<Utc>,
    pub status: String,
    pub price_amount: Option<String>,
    pub price_currency: Option<String>,
    /// `not_required`, `unpaid` or `paid`.
    pub payment_status: String,
    pub payment_mode: Option<PaymentMode>,
    pub payment_due_at: Option<DateTime<Utc>>,
}

/// One QNB payment link.
#[derive(Debug, Clone, FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub booking_id: Uuid,
    pub merchant_txn_id: String,
    pub ipg_txn_id: Option<String>,
    pub pay_url: Option<String>,
    pub amount: String,
    pub currency: String,
    pub last_failure: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
}
