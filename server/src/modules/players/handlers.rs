use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use super::{
    dto::{ListQuery, PlayerInput},
    model::Player,
    service,
};
use crate::{
    error::AppResult,
    modules::{chats, push, stats},
    shared::pagination::Paginated,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Paginated<Player>>> {
    Ok(Json(service::list(&state.db, query).await?))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> AppResult<Json<Player>> {
    Ok(Json(service::get(&state.db, id).await?))
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<PlayerInput>,
) -> AppResult<(StatusCode, Json<Player>)> {
    let player = service::create(&state.db, input).await?;
    // Stats from earlier uploads that listed this name become theirs.
    stats::link_unclaimed_records(&state, player.id, &player.playername).await;
    Ok((StatusCode::CREATED, Json(player)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<PlayerInput>,
) -> AppResult<Json<Player>> {
    let password_reset = input.password.as_deref().is_some_and(|p| !p.is_empty());
    let player = service::update(&state.db, id, input).await?;
    if password_reset {
        // Their tokens are now rejected; close live chat sockets too.
        chats::disconnect_player(&state, id);
        // …and stop push notifications to their phones.
        push::forget_player(&state, id).await;
    }
    Ok(Json(player))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<Uuid>) -> AppResult<StatusCode> {
    service::delete(&state.db, id).await?;
    chats::disconnect_player(&state, id);
    Ok(StatusCode::NO_CONTENT)
}
