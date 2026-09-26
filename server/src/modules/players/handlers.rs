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
use crate::{error::AppResult, shared::pagination::Paginated, state::AppState};

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
    Ok((StatusCode::CREATED, Json(player)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<PlayerInput>,
) -> AppResult<Json<Player>> {
    Ok(Json(service::update(&state.db, id, input).await?))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<Uuid>) -> AppResult<StatusCode> {
    service::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
