//! Admin panel: staff chat with players as the "Havoc Admin" player, through the same
//! service as the app (so messages reach the app live, with push and read receipts).

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use super::{
    dto::{
        Conversation, MessagesPage, MessagesQuery, PlayersQuery, ReadResponse, SendMessageRequest,
    },
    model::{ChatMessage, ChatPlayer},
    service,
};
use crate::{
    error::AppResult,
    modules::{auth::AuthUser, players::HAVOC_ADMIN_ID},
    shared::pagination::Paginated,
    state::AppState,
};

/// Every player (except Havoc Admin), by name.
pub async fn players(
    State(state): State<AppState>,
    _admin: AuthUser,
    Query(query): Query<PlayersQuery>,
) -> AppResult<Json<Paginated<ChatPlayer>>> {
    Ok(Json(
        service::list_players(&state, HAVOC_ADMIN_ID, query).await?,
    ))
}

/// Havoc Admin's conversations, most recent first.
pub async fn conversations(
    State(state): State<AppState>,
    _admin: AuthUser,
) -> AppResult<Json<Vec<Conversation>>> {
    Ok(Json(
        service::list_conversations(&state, HAVOC_ADMIN_ID).await?,
    ))
}

pub async fn messages(
    State(state): State<AppState>,
    _admin: AuthUser,
    Path(player_id): Path<Uuid>,
    Query(query): Query<MessagesQuery>,
) -> AppResult<Json<MessagesPage>> {
    Ok(Json(
        service::list_messages(&state, HAVOC_ADMIN_ID, player_id, query).await?,
    ))
}

/// Sends as Havoc Admin.
pub async fn send(
    State(state): State<AppState>,
    admin: AuthUser,
    Path(player_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> AppResult<(StatusCode, Json<ChatMessage>)> {
    let message = service::send_message(&state, HAVOC_ADMIN_ID, player_id, &req.body, None).await?;
    tracing::info!(admin = %admin.id, %player_id, "admin chat message sent");
    Ok((StatusCode::CREATED, Json(message)))
}

pub async fn read(
    State(state): State<AppState>,
    _admin: AuthUser,
    Path(player_id): Path<Uuid>,
) -> AppResult<Json<ReadResponse>> {
    let read_at = service::mark_read(&state, HAVOC_ADMIN_ID, player_id).await?;
    Ok(Json(ReadResponse { read_at }))
}
