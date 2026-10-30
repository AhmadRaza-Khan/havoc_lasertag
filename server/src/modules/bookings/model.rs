use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::integrations::bookeo;

#[derive(Debug, FromRow)]
pub struct BookingRow {
    pub id: Uuid,
    pub player_id: Uuid,
    pub bookeo_booking_number: String,
    pub product_id: String,
    pub product_name: String,
    pub event_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub num_people: i32,
    /// Other registered players in the group (ids, in the order they were added).
    pub other_players: Vec<Uuid>,
    pub status: String,
    pub price_amount: Option<String>,
    pub price_currency: Option<String>,
    /// Add-ons, as sent to Bookeo.
    pub options: sqlx::types::Json<Vec<bookeo::BookingOption>>,
    /// `not_required`, `unpaid`, `partially_paid` or `paid` (online payment of app bookings).
    pub payment_status: String,
    /// Paid online so far, e.g. "120.00".
    pub amount_paid: String,
    /// Unpaid bookings are canceled after this.
    pub payment_due_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A booking joined with the player who owns it (admin views).
#[derive(Debug, FromRow)]
pub struct BookingWithPlayer {
    #[sqlx(flatten)]
    pub booking: BookingRow,
    pub player_full_name: String,
    pub player_playername: String,
}

/// Local status derived from Bookeo's booking flags.
pub fn status_of(b: &bookeo::Booking) -> &'static str {
    if b.canceled == Some(true) {
        "canceled"
    } else if b.accepted == Some(false) {
        "pending"
    } else {
        "confirmed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn booking(canceled: Option<bool>, accepted: Option<bool>) -> bookeo::Booking {
        bookeo::Booking {
            booking_number: "1".into(),
            event_id: None,
            product_id: None,
            product_name: None,
            start_time: None,
            end_time: None,
            canceled,
            accepted,
            price: None,
            participants: None,
            options: None,
        }
    }

    #[test]
    fn maps_status() {
        assert_eq!(status_of(&booking(None, None)), "confirmed");
        assert_eq!(status_of(&booking(Some(false), Some(true))), "confirmed");
        assert_eq!(status_of(&booking(Some(false), Some(false))), "pending");
        assert_eq!(status_of(&booking(Some(true), Some(false))), "canceled");
    }
}
