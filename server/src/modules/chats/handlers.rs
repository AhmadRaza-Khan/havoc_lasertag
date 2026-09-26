//! REST endpoints: loading data, plus send/read as a fallback when the socket is down.

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
    error::AppResult, modules::player_auth::AuthPlayer, shared::pagination::Paginated,
    state::AppState,
};

pub async fn players(
    State(state): State<AppState>,
    me: AuthPlayer,
    Query(query): Query<PlayersQuery>,
) -> AppResult<Json<Paginated<ChatPlayer>>> {
    Ok(Json(service::list_players(&state, me.id, query).await?))
}

pub async fn player(
    State(state): State<AppState>,
    _me: AuthPlayer,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ChatPlayer>> {
    Ok(Json(service::get_player(&state, id).await?))
}

pub async fn conversations(
    State(state): State<AppState>,
    me: AuthPlayer,
) -> AppResult<Json<Vec<Conversation>>> {
    Ok(Json(service::list_conversations(&state, me.id).await?))
}

pub async fn messages(
    State(state): State<AppState>,
    me: AuthPlayer,
    Path(player_id): Path<Uuid>,
    Query(query): Query<MessagesQuery>,
) -> AppResult<Json<MessagesPage>> {
    Ok(Json(
        service::list_messages(&state, me.id, player_id, query).await?,
    ))
}

pub async fn send(
    State(state): State<AppState>,
    me: AuthPlayer,
    Path(player_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> AppResult<(StatusCode, Json<ChatMessage>)> {
    let message = service::send_message(&state, me.id, player_id, &req.body, None).await?;
    Ok((StatusCode::CREATED, Json(message)))
}

pub async fn read(
    State(state): State<AppState>,
    me: AuthPlayer,
    Path(player_id): Path<Uuid>,
) -> AppResult<Json<ReadResponse>> {
    let read_at = service::mark_read(&state, me.id, player_id).await?;
    Ok(Json(ReadResponse { read_at }))
}
