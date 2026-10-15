use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::model::{ChatMessage, ChatPlayer, ConversationRow};
use crate::error::{AppError, AppResult};

pub const MAX_BODY_CHARS: usize = 2000;
const DEFAULT_MESSAGES: i64 = 30;
const MAX_MESSAGES: i64 = 100;

/// Trims a message body and checks its length.
pub fn validate_body(body: &str) -> AppResult<String> {
    let body = body.trim();
    if body.is_empty() {
        return Err(AppError::BadRequest("Message can't be empty".into()));
    }
    if body.chars().count() > MAX_BODY_CHARS {
        return Err(AppError::BadRequest(format!(
            "Message can be at most {MAX_BODY_CHARS} characters"
        )));
    }
    Ok(body.to_string())
}

#[derive(Debug, Deserialize)]
pub struct PlayersQuery {
    /// Matches playername or full name.
    pub search: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MessagesQuery {
    /// Message id: return messages older than it (for loading earlier history).
    pub before: Option<Uuid>,
    pub limit: Option<i64>,
}

impl MessagesQuery {
    pub fn limit(&self) -> i64 {
        self.limit
            .unwrap_or(DEFAULT_MESSAGES)
            .clamp(1, MAX_MESSAGES)
    }
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct MessagesPage {
    /// Newest first.
    pub items: Vec<ChatMessage>,
    /// More, older messages exist: request again with `before` = the last item's id.
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
pub struct Conversation {
    pub id: Uuid,
    pub player: ChatPlayer,
    pub last_message: Option<LastMessage>,
    pub unread_count: i64,
}

#[derive(Debug, Serialize)]
pub struct LastMessage {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ReadResponse {
    /// When the messages were marked read; `null` if there was nothing unread.
    pub read_at: Option<DateTime<Utc>>,
}

impl Conversation {
    pub fn from_row(r: ConversationRow, online: bool) -> Self {
        let last_message = match (
            r.last_message_id,
            r.last_message_sender_id,
            r.last_message_body,
            r.last_message_created_at,
        ) {
            (Some(id), Some(sender_id), Some(body), Some(created_at)) => Some(LastMessage {
                id,
                sender_id,
                body,
                created_at,
                read_at: r.last_message_read_at,
            }),
            _ => None,
        };
        Self {
            id: r.id,
            player: ChatPlayer {
                id: r.other_id,
                full_name: r.other_full_name,
                playername: r.other_playername,
                last_seen_at: r.other_last_seen_at,
                avatar_url: r.other_avatar_url,
                online,
            },
            last_message,
            unread_count: r.unread_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_bodies() {
        assert_eq!(validate_body("  hi  ").unwrap(), "hi");
        assert!(validate_body("   ").is_err());
        assert!(validate_body(&"x".repeat(MAX_BODY_CHARS)).is_ok());
        assert!(validate_body(&"x".repeat(MAX_BODY_CHARS + 1)).is_err());
        // counts characters, not bytes
        assert!(validate_body(&"ع".repeat(MAX_BODY_CHARS)).is_ok());
    }

    #[test]
    fn clamps_limit() {
        let q = |limit| MessagesQuery {
            before: None,
            limit,
        };
        assert_eq!(q(None).limit(), 30);
        assert_eq!(q(Some(0)).limit(), 1);
        assert_eq!(q(Some(1000)).limit(), 100);
    }
}
