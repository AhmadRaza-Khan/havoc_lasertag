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
