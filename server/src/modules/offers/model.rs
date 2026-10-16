use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow)]
pub struct Offer {
    pub id: Uuid,
    /// May be empty: an offer can be just an image.
    pub title: String,
    /// May be empty.
    pub description: String,
    pub is_active: bool,
    /// First day the offer shows (venue's local date); `None` = no start limit.
    pub starts_on: Option<NaiveDate>,
    /// Last day the offer shows, inclusive; `None` = never expires.
    pub ends_on: Option<NaiveDate>,
    /// API path of the offer's image, if it has one (versioned, so it can be cached for good).
    pub image_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Offer {
    pub fn has_text(&self) -> bool {
        !self.title.is_empty() || !self.description.is_empty()
    }
}
