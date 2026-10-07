//! Chat operations shared by the REST endpoints and the WebSocket. Every write also pushes
//! the matching real-time events through the hub.

use uuid::Uuid;

use super::{
    dto::{Conversation, MessagesPage, MessagesQuery, PlayersQuery, validate_body},
    events::ServerEvent,
    model::{ChatMessage, ChatPlayer},
    repository,
};
use crate::{
    error::{AppError, AppResult},
    shared::pagination::{PageParams, Paginated, like_pattern},
    state::AppState,
};

pub async fn list_players(
    state: &AppState,
    me: Uuid,
    query: PlayersQuery,
) -> AppResult<Paginated<ChatPlayer>> {
    let params = PageParams {
        page: query.page,
        per_page: query.per_page,
    };
    let pattern = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(like_pattern);

    let (mut players, total) = repository::list_players(
        &state.db,
        me,
        pattern.as_deref(),
        params.per_page(),
        params.offset(),
    )
    .await?;
    for p in &mut players {
        p.online = state.chat.is_online(p.id);
    }
    Ok(Paginated::new(players, total, &params))
}

pub async fn get_player(state: &AppState, id: Uuid) -> AppResult<ChatPlayer> {
    let mut player = repository::find_player(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Player not found".into()))?;
    player.online = state.chat.is_online(player.id);
    Ok(player)
}

pub async fn list_conversations(state: &AppState, me: Uuid) -> AppResult<Vec<Conversation>> {
    let rows = repository::list_conversations(&state.db, me).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let online = state.chat.is_online(r.other_id);
            Conversation::from_row(r, online)
        })
        .collect())
}

pub async fn list_messages(
    state: &AppState,
    me: Uuid,
    other: Uuid,
    query: MessagesQuery,
) -> AppResult<MessagesPage> {
    let limit = query.limit();
    // Fetch one extra to know whether older messages exist.
    let mut items =
        repository::list_messages(&state.db, me, other, query.before, limit + 1).await?;
    let has_more = items.len() as i64 > limit;
    items.truncate(limit as usize);
    Ok(MessagesPage { items, has_more })
}

/// Stores a message and delivers it to both players' connected devices.
pub async fn send_message(
    state: &AppState,
    from: Uuid,
    to: Uuid,
    body: &str,
    client_id: Option<String>,
) -> AppResult<ChatMessage> {
    if from == to {
        return Err(AppError::BadRequest("You can't message yourself".into()));
    }
    let body = validate_body(body)?;
    if repository::find_player(&state.db, to).await?.is_none() {
        return Err(AppError::NotFound("Player not found".into()));
    }

    let message = repository::insert_message(&state.db, from, to, &body).await?;

    // The app keeps its socket open only while it's in the foreground: anyone not connected
    // gets a push notification instead.
    if !state.chat.is_online(to)
        && let Some(sender) = repository::find_player(&state.db, from).await?
    {
        crate::modules::push::notify_chat_message(state, to, from, &sender.playername, &body);
    }

    state.chat.send(
        to,
        &ServerEvent::Message {
            message: message.clone(),
            client_id: None,
        },
    );
    state.chat.send(
        from,
        &ServerEvent::Message {
            message: message.clone(),
            client_id,
        },
    );
    Ok(message)
}

/// Marks what `other` sent `me` as read and tells `other` (read receipts).
pub async fn mark_read(
    state: &AppState,
    me: Uuid,
    other: Uuid,
) -> AppResult<Option<chrono::DateTime<chrono::Utc>>> {
    let read_at = repository::mark_read(&state.db, me, other).await?;
    if let Some(read_at) = read_at {
        state
            .chat
            .send(other, &ServerEvent::Read { by: me, read_at });
    }
    Ok(read_at)
}

/// Typing indicators are relayed live and never stored.
pub fn typing(state: &AppState, from: Uuid, to: Uuid, is_typing: bool) {
    if from != to {
        state
            .chat
            .send(to, &ServerEvent::Typing { from, is_typing });
    }
}

/// Closes the player's open chat sockets and tells everyone they're offline. Call when the
/// account is deleted or its password is reset, so old sessions lose live access at once.
pub fn disconnect_player(state: &AppState, player_id: Uuid) {
    if state.chat.disconnect(player_id) {
        state.chat.broadcast_except(
            player_id,
            &ServerEvent::Presence {
                player_id,
                online: false,
                last_seen_at: Some(chrono::Utc::now()),
            },
        );
        tracing::info!(%player_id, "chat sockets closed (account changed)");
    }
}
