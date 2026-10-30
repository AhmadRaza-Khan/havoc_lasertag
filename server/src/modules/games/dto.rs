use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::{
    integrations::bookeo::{self, Money, OptionCommon, Product},
    shared::html,
};

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
    /// Add-ons offered when booking (Bookeo "options"), in display order. Their prices are
    /// applied by Bookeo: the booking's price includes them.
    pub options: Vec<GameOption>,
}

/// An add-on for a booking. Send the chosen ones as `options: [{id, value}]`.
#[derive(Debug, Serialize)]
pub struct GameOption {
    pub id: String,
    /// `choice` (pick one of `values`), `number` (`min`..=`max`), `on_off` (`"true"`/`"false"`)
    /// or `text`.
    pub kind: &'static str,
    pub name: String,
    /// Plain text.
    pub description: Option<String>,
    /// Only staff may set it (not shown in the mobile app).
    pub staff_only: bool,
    /// Text options only: a value is required.
    pub required: bool,
    /// Choice options: the possible values (send the value's `id`).
    pub values: Vec<GameOptionValue>,
    pub min: Option<i32>,
    pub max: Option<i32>,
    /// What to preselect: a value id (choice), a number, or `"true"`/`"false"`.
    pub default: Option<String>,
    /// The add-on's picture, if admins set one (Settings → Add-ons).
    pub image_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GameOptionValue {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// The product's enabled add-ons, in Bookeo's display order.
pub fn game_options(p: &Product) -> Vec<GameOption> {
    let text = |d: &Option<String>| d.as_deref().map(html::to_text).filter(|t| !t.is_empty());
    let base = |c: &OptionCommon, kind: &'static str| GameOption {
        id: c.id.clone(),
        kind,
        name: c.name.trim().to_string(),
        description: text(&c.description),
        staff_only: !c.shown_to_customers,
        required: false,
        values: Vec::new(),
        min: None,
        max: None,
        default: None,
        image_url: None,
    };
    let mut out: Vec<(i32, GameOption)> = Vec::new();
    for o in &p.choice_options {
        out.push((
            o.common.index,
            GameOption {
                values: o
                    .values
                    .iter()
                    .map(|v| GameOptionValue {
                        id: v.id.clone(),
                        name: v.name.trim().to_string(),
                        description: text(&v.description),
                    })
                    .collect(),
                default: o.default_value_id.clone().filter(|d| !d.is_empty()),
                ..base(&o.common, "choice")
            },
        ));
    }
    for o in &p.number_options {
        out.push((
            o.common.index,
            GameOption {
                min: Some(o.min_value),
                max: Some(o.max_value),
                default: Some(o.default_value.to_string()),
                ..base(&o.common, "number")
            },
        ));
    }
    for o in &p.on_off_options {
        out.push((
            o.common.index,
            GameOption {
                default: Some(o.default_state.to_string()),
                ..base(&o.common, "on_off")
            },
        ));
    }
    for o in &p.text_options {
        out.push((
            o.common.index,
            GameOption {
                required: o.required,
                ..base(&o.common, "text")
            },
        ));
    }
    let enabled = |id: &str| {
        let all = p
            .choice_options
            .iter()
            .map(|o| &o.common)
            .chain(p.number_options.iter().map(|o| &o.common))
            .chain(p.on_off_options.iter().map(|o| &o.common))
            .chain(p.text_options.iter().map(|o| &o.common));
        all.into_iter().any(|c| c.id == id && c.enabled)
    };
    out.retain(|(_, o)| enabled(&o.id));
    out.sort_by_key(|(i, _)| *i);
    out.into_iter().map(|(_, o)| o).collect()
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
            options: game_options(p),
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

#[cfg(test)]
mod addon_tests {
    use super::*;

    #[test]
    fn lists_enabled_addons_in_order_as_text() {
        let p: Product = serde_json::from_value(serde_json::json!({
            "productId": "P1", "name": "Open Session", "type": "fixed",
            "onOffOptions": [{"id": "B", "name": "Search & Destroy ", "index": 1,
                "description": "<p><strong>Add the mission &amp; win.</strong></p>",
                "shownToCustomers": true, "enabled": true, "defaultState": false}],
            "choiceOptions": [{"id": "A", "name": "UPGRADE", "index": 0, "shownToCustomers": true, "enabled": true,
                "values": [{"id": "A1", "name": "Full", "description": "Extra ammo<br />+ grenade"}]}],
            "textOptions": [{"id": "C", "name": "Old", "index": 2, "shownToCustomers": true, "enabled": false, "required": false}]
        }))
        .unwrap();
        let o = game_options(&p);
        assert_eq!(
            o.iter()
                .map(|o| (o.id.as_str(), o.kind))
                .collect::<Vec<_>>(),
            [("A", "choice"), ("B", "on_off")]
        );
        assert_eq!(
            o[0].values[0].description.as_deref(),
            Some("Extra ammo\n+ grenade")
        );
        assert_eq!(o[1].name, "Search & Destroy");
        assert_eq!(o[1].description.as_deref(), Some("Add the mission & win."));
        assert_eq!(o[1].default.as_deref(), Some("false"));
    }
}
