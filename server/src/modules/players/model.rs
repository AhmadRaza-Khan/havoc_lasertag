use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// A player as returned to clients. The password hash is never selected into this type.
#[derive(Debug, Serialize, FromRow)]
pub struct Player {
    pub id: Uuid,
    pub full_name: String,
    pub playername: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Profile picture (`/api/avatars/{id}?v=…`, prefix the API origin), if they uploaded one.
    pub avatar_url: Option<String>,
}
