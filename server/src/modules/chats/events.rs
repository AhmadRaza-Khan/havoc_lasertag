//! The WebSocket protocol. Every frame is a JSON text message with a `type` field.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::model::ChatMessage;

/// Sent by the app.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientEvent {
    /// Send a message. `client_id` is echoed back so the app can match its optimistic message.
    Send {
        to: Uuid,
        body: String,
        client_id: Option<String>,
    },
    /// Started/stopped typing to `to`. Apps should re-send `true` every few seconds while typing.
    Typing { to: Uuid, is_typing: bool },
    /// Mark everything `with` sent me as read.
    Read { with: Uuid },
    /// Application-level keepalive (WebSocket ping frames work too).
    Ping,
}

/// Sent by the server.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    /// First event after connecting.
    Ready {
        player_id: Uuid,
        online_player_ids: Vec<Uuid>,
    },
    /// A new message, to both the recipient and the sender (all of their devices).
    Message {
        message: ChatMessage,
        /// Only on the sender's copy, when the send came over WebSocket with a `client_id`.
        #[serde(skip_serializing_if = "Option::is_none")]
        client_id: Option<String>,
    },
    /// `from` started/stopped typing to you. Treat as stopped if not refreshed for ~6 seconds.
    Typing {
        from: Uuid,
        is_typing: bool,
    },
    /// `by` read the messages you sent them, up to `read_at`.
    Read {
        by: Uuid,
        read_at: DateTime<Utc>,
    },
    /// A player came online or went offline. Broadcast to everyone connected.
    Presence {
        player_id: Uuid,
        online: bool,
        last_seen_at: Option<DateTime<Utc>>,
    },
    Pong,
    /// A client event failed. Only sent to the connection that caused it.
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        client_id: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_client_events() {
        let e: ClientEvent = serde_json::from_str(
            r#"{"type":"send","to":"7f1c1f4e-8a0e-4b8e-9a39-2d6f0c1e2b3a","body":"hi","client_id":"c1"}"#,
        )
        .unwrap();
        assert!(matches!(
            e,
            ClientEvent::Send {
                client_id: Some(_),
                ..
            }
        ));
        let e: ClientEvent = serde_json::from_str(r#"{"type":"ping"}"#).unwrap();
        assert!(matches!(e, ClientEvent::Ping));
        assert!(serde_json::from_str::<ClientEvent>(r#"{"type":"nope"}"#).is_err());
    }

    #[test]
    fn serializes_server_events() {
        let json = serde_json::to_string(&ServerEvent::Typing {
            from: Uuid::nil(),
            is_typing: true,
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"type":"typing","from":"00000000-0000-0000-0000-000000000000","is_typing":true}"#
        );
        let json = serde_json::to_string(&ServerEvent::Error {
            message: "x".into(),
            client_id: None,
        })
        .unwrap();
        assert_eq!(json, r#"{"type":"error","message":"x"}"#);
    }
}
