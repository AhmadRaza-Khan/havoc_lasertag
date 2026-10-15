use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Another player as seen in chat: no contact details, only identity and presence.
#[derive(Debug, Serialize, FromRow)]
pub struct ChatPlayer {
    pub id: Uuid,
    pub full_name: String,
    pub playername: String,
    pub last_seen_at: Option<DateTime<Utc>>,
    /// Profile picture path, if any.
    pub avatar_url: Option<String>,
    /// Filled in from the live connection hub, not the database.
    #[sqlx(skip)]
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ChatMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

/// Row of the conversation list query.
#[derive(Debug, FromRow)]
pub struct ConversationRow {
    pub id: Uuid,
    pub other_id: Uuid,
    pub other_full_name: String,
    pub other_playername: String,
    pub other_last_seen_at: Option<DateTime<Utc>>,
    pub other_avatar_url: Option<String>,
    pub last_message_id: Option<Uuid>,
    pub last_message_sender_id: Option<Uuid>,
    pub last_message_body: Option<String>,
    pub last_message_created_at: Option<DateTime<Utc>>,
    pub last_message_read_at: Option<DateTime<Utc>>,
    pub unread_count: i64,
}
