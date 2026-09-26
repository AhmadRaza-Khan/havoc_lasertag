use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::integrations::bookeo::{self, Money, Product};

#[derive(Debug, Serialize)]
pub struct Game {
    pub product_id: String,
    pub name: String,
    pub description: Option<String>,
    pub images: Vec<String>,
    pub duration_minutes: Option<i32>,
    pub price: Option<GamePrice>,
    pub min_people: Option<i32>,
    pub max_people: Option<i32>,
    /// Bookings for this game are held as "pending" until staff accept them.
    pub requires_approval: bool,
}

#[derive(Debug, Serialize)]
pub struct GamePrice {
    pub amount: String,
    pub currency: String,
    /// true = price per person; false = price per booking.
    pub per_person: bool,
}

impl From<&Product> for Game {
    fn from(p: &Product) -> Self {
        let (min_people, max_people) = people_limits(p).unzip();
        Self {
            product_id: p.product_id.clone(),
            name: p.name.trim().to_string(),
            description: p.description.clone(),
            images: p.images.iter().map(|i| i.url.clone()).collect(),
            duration_minutes: p.duration.as_ref().map(duration_minutes),
            price: p.default_rates.first().map(|r| {
                let Money { amount, currency } = r.price.clone();
                GamePrice {
                    amount,
                    currency,
                    per_person: r.people_category_id.is_some(),
                }
            }),
            min_people,
            max_people,
            requires_approval: p.accept_deny.unwrap_or(false),
        }
    }
}

/// Group size limits: the total-people limit if Bookeo defines one, else the category limit.
pub fn people_limits(p: &Product) -> Option<(i32, i32)> {
    p.booking_limits
        .iter()
        .find(|l| l.people_category_id.is_none())
        .or_else(|| p.booking_limits.first())
        .map(|l| (l.min, l.max))
}

fn duration_minutes(d: &bookeo::Duration) -> i32 {
    d.days * 24 * 60 + d.hours * 60 + d.minutes
}

#[derive(Debug, Deserialize)]
pub struct SlotsQuery {
    /// YYYY-MM-DD, inclusive, business timezone. Defaults to today.
    pub from: Option<NaiveDate>,
    /// YYYY-MM-DD, inclusive, business timezone. Defaults to `from` (or today).
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct SlotsResponse {
    pub product_id: String,
    pub game_name: String,
    pub timezone: String,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub slots: Vec<AvailableSlot>,
}

#[derive(Debug, Serialize)]
pub struct AvailableSlot {
    /// Pass this to `POST /api/player/bookings` to book the slot.
    pub event_id: String,
    pub start_time: DateTime<FixedOffset>,
    pub end_time: DateTime<FixedOffset>,
    pub seats_available: i32,
}
