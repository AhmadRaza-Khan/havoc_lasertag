//! Subsets of the Bookeo schemas we use. Unknown fields are ignored and most fields are optional,
//! so additions on Bookeo's side don't break deserialization.

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Page<T> {
    pub info: Option<PageInfo>,
    #[serde(default = "Vec::new")]
    pub data: Vec<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub total_pages: Option<i32>,
    pub current_page: Option<i32>,
    pub page_navigation_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub product_id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub images: Vec<Image>,
    #[serde(rename = "type")]
    pub product_type: String,
    pub duration: Option<Duration>,
    #[serde(default)]
    pub booking_limits: Vec<BookingLimit>,
    #[serde(default)]
    pub default_rates: Vec<PriceRate>,
    pub api_bookings_allowed: Option<bool>,
    pub shown_to_customers: Option<bool>,
    pub accept_deny: Option<bool>,
    /// Add-ons ("options"), including the business's global ones, which Bookeo lists on every
    /// product with product-specific ids. Each list is absent when empty.
    #[serde(default)]
    pub choice_options: Vec<ChoiceOption>,
    #[serde(default)]
    pub number_options: Vec<NumberOption>,
    #[serde(default)]
    pub on_off_options: Vec<OnOffOption>,
    #[serde(default)]
    pub text_options: Vec<TextOption>,
}

/// Fields every kind of option has.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionCommon {
    pub id: String,
    pub name: String,
    /// Display order across all kinds of options.
    pub index: i32,
    /// HTML from Bookeo's editor.
    pub description: Option<String>,
    /// False: staff-only, not offered to customers.
    pub shown_to_customers: bool,
    /// False: not for new bookings.
    pub enabled: bool,
}

/// Pick one value from a list.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceOption {
    #[serde(flatten)]
    pub common: OptionCommon,
    #[serde(default)]
    pub values: Vec<ChoiceOptionValue>,
    pub default_value_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChoiceOptionValue {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberOption {
    #[serde(flatten)]
    pub common: OptionCommon,
    pub min_value: i32,
    pub max_value: i32,
    pub default_value: i32,
}

/// A checkbox.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnOffOption {
    #[serde(flatten)]
    pub common: OptionCommon,
    pub default_state: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextOption {
    #[serde(flatten)]
    pub common: OptionCommon,
    pub required: bool,
}

/// An option value on a booking. For choice options `value` is the chosen value's *name*;
/// for checkboxes "true"/"false".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BookingOption {
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Image {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Duration {
    #[serde(default)]
    pub days: i32,
    #[serde(default)]
    pub hours: i32,
    #[serde(default)]
    pub minutes: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookingLimit {
    /// Absent = limit on the total number of people in the booking.
    pub people_category_id: Option<String>,
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceRate {
    /// Absent = price is per booking rather than per person.
    pub people_category_id: Option<String>,
    pub price: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Money {
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub product_id: String,
    pub event_id: String,
    pub start_time: DateTime<FixedOffset>,
    pub end_time: DateTime<FixedOffset>,
    pub num_seats_available: Option<i32>,
    pub num_seats_available_as_dropin: Option<i32>,
    pub private_event: Option<bool>,
}

impl Slot {
    pub fn seats_available(&self) -> i32 {
        self.num_seats_available
            .or(self.num_seats_available_as_dropin)
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Booking {
    pub booking_number: String,
    pub event_id: Option<String>,
    pub product_id: Option<String>,
    pub product_name: Option<String>,
    pub start_time: Option<DateTime<FixedOffset>>,
    pub end_time: Option<DateTime<FixedOffset>>,
    pub canceled: Option<bool>,
    pub accepted: Option<bool>,
    pub price: Option<Price>,
    pub participants: Option<Participants>,
    /// Absent when the booking has none.
    #[serde(default)]
    pub options: Option<Vec<BookingOption>>,
}

impl Booking {
    /// Total people across all categories, if Bookeo returned participants.
    pub fn num_people(&self) -> Option<i32> {
        self.participants
            .as_ref()
            .map(|p| p.numbers.iter().map(|n| n.number).sum())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub total_gross: Option<Money>,
}

/// A payment recorded on a booking.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPayment {
    pub received_time: String,
    /// Shown to the customer, e.g. "Online payment".
    pub reason: String,
    /// Not shown to customers; we put the gateway's transaction id here.
    pub comment: String,
    pub amount: Money,
    /// One of Bookeo's methods: "creditCard", "cash", "other", ...
    pub payment_method: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    pub id: String,
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Customer {
    pub id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCustomer {
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_address: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub phone_numbers: Vec<PhoneNumber>,
}

#[derive(Debug, Serialize)]
pub struct PhoneNumber {
    pub number: String,
    /// One of "mobile", "work", "home", "fax".
    #[serde(rename = "type")]
    pub kind: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBooking {
    pub product_id: String,
    pub event_id: String,
    pub customer_id: String,
    /// Our booking id, so the booking can be traced back from Bookeo's side.
    pub external_ref: String,
    pub participants: Participants,
    pub source: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<BookingOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participants {
    pub numbers: Vec<PeopleNumber>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeopleNumber {
    pub people_category_id: String,
    pub number: i32,
}

/// A webhook registered with Bookeo (`GET /webhooks`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Webhook {
    pub id: Option<String>,
    pub url: String,
    pub domain: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// Set when Bookeo stopped calling it (unreachable for over 24 h); register it again.
    pub blocked_time: Option<String>,
    pub blocked_reason: Option<String>,
    /// Set while Bookeo is backing off after failed deliveries.
    pub no_delivery_before: Option<String>,
    pub delivery_delay_reason: Option<String>,
}

/// `POST /webhooks`: replaces any webhook for the same domain and type on this API key.
#[derive(Debug, Serialize)]
pub struct NewWebhook<'a> {
    pub url: &'a str,
    /// e.g. "bookings"
    pub domain: &'a str,
    /// "created", "updated" or "deleted"
    #[serde(rename = "type")]
    pub kind: &'a str,
}

/// What Bookeo POSTs to a webhook URL.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookMessage {
    /// For bookings, the booking number.
    pub item_id: Option<String>,
    /// The item as `GET /bookings/{itemId}` returns it (may be absent if it was deleted).
    pub item: Option<serde_json::Value>,
}
