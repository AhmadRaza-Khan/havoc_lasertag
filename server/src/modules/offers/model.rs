use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow)]
pub struct Offer {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub is_active: bool,
    /// First day the offer shows (venue's local date); `None` = no start limit.
    pub starts_on: Option<NaiveDate>,
    /// Last day the offer shows, inclusive; `None` = never expires.
    pub ends_on: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
