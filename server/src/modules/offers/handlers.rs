use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use super::{
    dto::{ListQuery, OfferInput},
    model::Offer,
    service,
};
use crate::{error::AppResult, shared::pagination::Paginated, state::AppState};

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Paginated<Offer>>> {
    Ok(Json(service::list(&state.db, query).await?))
}

/// Mobile app: the offers live today.
pub async fn live(State(state): State<AppState>) -> AppResult<Json<Vec<Offer>>> {
    Ok(Json(
        service::live(&state.db, state.config.bookeo.timezone).await?,
    ))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> AppResult<Json<Offer>> {
    Ok(Json(service::get(&state.db, id).await?))
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<OfferInput>,
) -> AppResult<(StatusCode, Json<Offer>)> {
    Ok((
        StatusCode::CREATED,
        Json(service::create(&state.db, input).await?),
    ))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<OfferInput>,
) -> AppResult<Json<Offer>> {
    Ok(Json(service::update(&state.db, id, input).await?))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<Uuid>) -> AppResult<StatusCode> {
    service::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
