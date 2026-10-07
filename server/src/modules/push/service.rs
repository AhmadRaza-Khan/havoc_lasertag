use serde_json::json;
use uuid::Uuid;

use super::repository;
use crate::{
    error::{AppError, AppResult},
    integrations::expo_push::{Delivery, PushMessage, is_expo_token},
    state::AppState,
};

/// Longest message text shown in a notification.
const PREVIEW_CHARS: usize = 140;

pub async fn register(
    state: &AppState,
    player_id: Uuid,
    token: &str,
    platform: &str,
) -> AppResult<()> {
    if !is_expo_token(token) {
        return Err(AppError::BadRequest("Not an Expo push token".into()));
    }
    if !matches!(platform, "ios" | "android") {
        return Err(AppError::BadRequest(
            "platform must be ios or android".into(),
        ));
    }
    repository::upsert(&state.db, token, player_id, platform).await?;
    Ok(())
}

pub async fn unregister(state: &AppState, player_id: Uuid, token: &str) -> AppResult<()> {
    repository::delete(&state.db, token, player_id).await?;
    Ok(())
}

/// Stops pushes to a player's phones (password reset: their sessions just ended).
pub async fn forget_player(state: &AppState, player_id: Uuid) {
    match repository::delete_for_player(&state.db, player_id).await {
        Ok(n) if n > 0 => tracing::info!(%player_id, devices = n, "push tokens removed"),
        Ok(_) => {}
        Err(e) => tracing::error!(%player_id, "could not remove push tokens: {e}"),
    }
}

/// Pushes a new chat message to the recipient's phones, in the background.
pub fn notify_chat_message(
    state: &AppState,
    to: Uuid,
    from: Uuid,
    from_playername: &str,
    body: &str,
) {
    let state = state.clone();
    let title = from_playername.to_string();
    let preview = preview(body);
    tokio::spawn(async move {
        let tokens = match repository::tokens_for(&state.db, to).await {
            Ok(t) if !t.is_empty() => t,
            Ok(_) => return,
            Err(e) => return tracing::error!(player_id = %to, "loading push tokens: {e}"),
        };
        let messages: Vec<PushMessage> = tokens
            .iter()
            .map(|t| PushMessage {
                to: t.clone(),
                title: title.clone(),
                body: preview.clone(),
                data: json!({ "type": "chat_message", "player_id": from }),
                sound: "default",
                channel_id: "messages",
                thread_id: Some(format!("chat-{from}")),
            })
            .collect();
        for (token, result) in tokens.iter().zip(state.push.send(&messages).await) {
            match result {
                Delivery::Accepted => {}
                Delivery::DeviceNotRegistered => {
                    if let Err(e) = repository::delete_token(&state.db, token).await {
                        tracing::error!("removing dead push token: {e}");
                    }
                }
                Delivery::Failed(why) => {
                    tracing::warn!(player_id = %to, "push not delivered: {why}")
                }
            }
        }
    });
}

fn preview(body: &str) -> String {
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= PREVIEW_CHARS {
        flat
    } else {
        format!(
            "{}…",
            flat.chars().take(PREVIEW_CHARS - 1).collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_are_one_line_and_short() {
        assert_eq!(preview("see you\n\nat  8"), "see you at 8");
        let long = preview(&"x".repeat(500));
        assert_eq!(long.chars().count(), PREVIEW_CHARS);
        assert!(long.ends_with('…'));
    }
}
