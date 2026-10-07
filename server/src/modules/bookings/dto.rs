use chrono::{DateTime, FixedOffset, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::model::{BookingRow, BookingWithPlayer};
use crate::error::{AppError, AppResult};

const MAX_PEOPLE: i32 = 100;

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

/// Player app: book a slot for yourself.
#[derive(Debug, Deserialize)]
pub struct CreateBookingRequest {
    pub product_id: String,
    /// From `GET .../games/{product_id}/slots`.
    pub event_id: String,
    /// Everyone in the group, including the booking's player and any `other_player_ids`.
    pub num_people: i32,
    /// Optional: other registered players joining the booking.
    #[serde(default)]
    pub other_player_ids: Vec<Uuid>,
}

impl CreateBookingRequest {
    pub fn validated(self) -> AppResult<Self> {
        let product_id = self.product_id.trim().to_string();
        let event_id = self.event_id.trim().to_string();
        if product_id.is_empty() || event_id.is_empty() {
            return Err(AppError::BadRequest(
                "product_id and event_id are required".into(),
            ));
        }
        validate_num_people(self.num_people)?;
        let other_player_ids = dedupe(self.other_player_ids);
        validate_group(self.num_people, other_player_ids.len())?;
        Ok(Self {
            product_id,
            event_id,
            num_people: self.num_people,
            other_player_ids,
        })
    }
}

/// Admin panel: book a slot for any player.
#[derive(Debug, Deserialize)]
pub struct AdminCreateBookingRequest {
    pub player_id: Uuid,
    #[serde(flatten)]
    pub booking: CreateBookingRequest,
}

/// Admin panel: move a booking to another slot of the same game and/or change the group size.
#[derive(Debug, Deserialize)]
pub struct UpdateBookingRequest {
    pub event_id: String,
    pub num_people: i32,
    /// Replaces the other players; omit to keep them as they are.
    pub other_player_ids: Option<Vec<Uuid>>,
}

impl UpdateBookingRequest {
    pub fn validated(self) -> AppResult<Self> {
        let event_id = self.event_id.trim().to_string();
        if event_id.is_empty() {
            return Err(AppError::BadRequest("event_id is required".into()));
        }
        validate_num_people(self.num_people)?;
        let other_player_ids = self.other_player_ids.map(dedupe);
        if let Some(others) = &other_player_ids {
            validate_group(self.num_people, others.len())?;
        }
        Ok(Self {
            event_id,
            num_people: self.num_people,
            other_player_ids,
        })
    }
}

/// The group is the booking's player plus the other players (plus any unregistered guests).
pub fn validate_group(num_people: i32, others: usize) -> AppResult<()> {
    let minimum = others as i32 + 1;
    if num_people < minimum {
        return Err(AppError::BadRequest(format!(
            "Number of people must be at least {minimum} (the player plus {others} other player{})",
            if others == 1 { "" } else { "s" }
        )));
    }
    Ok(())
}

/// Removes repeated ids, keeping the first occurrence's order.
fn dedupe(ids: Vec<Uuid>) -> Vec<Uuid> {
    let mut seen = std::collections::HashSet::new();
    ids.into_iter().filter(|id| seen.insert(*id)).collect()
}

fn validate_num_people(n: i32) -> AppResult<()> {
    if !(1..=MAX_PEOPLE).contains(&n) {
        return Err(AppError::BadRequest(format!(
            "num_people must be between 1 and {MAX_PEOPLE}"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum When {
    Upcoming,
    Past,
}

impl When {
    pub fn as_str(self) -> &'static str {
        match self {
            When::Upcoming => "upcoming",
            When::Past => "past",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Confirmed,
    Pending,
    Canceled,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Confirmed => "confirmed",
            Status::Pending => "pending",
            Status::Canceled => "canceled",
        }
    }
}

/// Player app: `?when=upcoming|past&page=&per_page=`.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// `upcoming` (soonest first) or `past` (latest first). Omit for all (latest first).
    pub when: Option<When>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Admin panel: all bookings with filters.
#[derive(Debug, Deserialize)]
pub struct AdminListQuery {
    /// Matches player name, playername or booking number.
    pub search: Option<String>,
    pub player_id: Option<Uuid>,
    pub when: Option<When>,
    pub status: Option<Status>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

// ---------------------------------------------------------------------------
// Responses
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct BookingResponse {
    pub id: Uuid,
    pub booking_number: String,
    pub product_id: String,
    pub game_name: String,
    pub event_id: String,
    /// In the business timezone's offset.
    pub start_time: DateTime<FixedOffset>,
    pub end_time: DateTime<FixedOffset>,
    pub num_people: i32,
    /// `confirmed`, `pending` (awaiting staff approval) or `canceled`.
    pub status: String,
    pub price: Option<BookingPrice>,
    /// `not_required` (nothing to pay online), `unpaid` or `paid`.
    pub payment_status: String,
    /// Unpaid bookings are canceled after this. Business timezone.
    pub payment_due_at: Option<DateTime<FixedOffset>>,
    /// Included in admin responses only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSummary>,
    /// Other registered players in the group (deleted players are left out).
    pub other_players: Vec<PlayerSummary>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BookingPrice {
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerSummary {
    pub id: Uuid,
    pub full_name: String,
    pub playername: String,
}

impl BookingResponse {
    pub fn from_row(row: BookingRow, tz: Tz) -> Self {
        let price = match (row.price_amount, row.price_currency) {
            (Some(amount), Some(currency)) => Some(BookingPrice { amount, currency }),
            _ => None,
        };
        Self {
            id: row.id,
            booking_number: row.bookeo_booking_number,
            product_id: row.product_id,
            game_name: row.product_name,
            event_id: row.event_id,
            start_time: row.start_time.with_timezone(&tz).fixed_offset(),
            end_time: row.end_time.with_timezone(&tz).fixed_offset(),
            num_people: row.num_people,
            status: row.status,
            price,
            payment_status: row.payment_status,
            payment_due_at: row
                .payment_due_at
                .map(|d| d.with_timezone(&tz).fixed_offset()),
            player: None,
            other_players: Vec::new(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }

    /// Fills `other_players` from `ids` (the row's ids, in order) using `lookup`.
    pub fn with_other_players(
        mut self,
        ids: &[Uuid],
        lookup: &std::collections::HashMap<Uuid, PlayerSummary>,
    ) -> Self {
        self.other_players = ids
            .iter()
            .filter_map(|id| lookup.get(id).cloned())
            .collect();
        self
    }

    pub fn from_row_with_player(row: BookingWithPlayer, tz: Tz) -> Self {
        let player = PlayerSummary {
            id: row.booking.player_id,
            full_name: row.player_full_name,
            playername: row.player_playername,
        };
        Self {
            player: Some(player),
            ..Self::from_row(row.booking, tz)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(product: &str, event: &str, n: i32) -> CreateBookingRequest {
        CreateBookingRequest {
            product_id: product.into(),
            event_id: event.into(),
            num_people: n,
            other_player_ids: Vec::new(),
        }
    }

    #[test]
    fn other_players_count_towards_the_group() {
        let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
        let mut r = req("p", "e", 3);
        r.other_player_ids = vec![a, b, a];
        let ok = r.validated().unwrap();
        assert_eq!(
            ok.other_player_ids,
            [a, b],
            "duplicates removed, order kept"
        );

        let mut too_small = req("p", "e", 2);
        too_small.other_player_ids = vec![a, b];
        let err = too_small.validated().unwrap_err();
        assert!(err.to_string().contains("at least 3"), "{err}");

        // More people than registered players is fine (unregistered guests).
        let mut guests = req("p", "e", 6);
        guests.other_player_ids = vec![a];
        assert!(guests.validated().is_ok());
    }

    #[test]
    fn validates_create_request() {
        assert!(req("p", "e", 2).validated().is_ok());
        assert!(req(" ", "e", 2).validated().is_err());
        assert!(req("p", "", 2).validated().is_err());
        assert!(req("p", "e", 0).validated().is_err());
        assert!(req("p", "e", 101).validated().is_err());
    }

    #[test]
    fn validates_update_request() {
        let ok = UpdateBookingRequest {
            event_id: " e ".into(),
            num_people: 3,
            other_player_ids: None,
        };
        assert_eq!(ok.validated().unwrap().event_id, "e");
        let bad = UpdateBookingRequest {
            event_id: "e".into(),
            num_people: 0,
            other_player_ids: None,
        };
        assert!(bad.validated().is_err());
    }

    #[test]
    fn admin_create_request_is_flat() {
        let r: AdminCreateBookingRequest = serde_json::from_str(
            r#"{"player_id":"7f1c1f4e-8a0e-4b8e-9a39-2d6f0c1e2b3a","product_id":"P","event_id":"E","num_people":4}"#,
        )
        .unwrap();
        assert_eq!(r.booking.num_people, 4);
    }
}
