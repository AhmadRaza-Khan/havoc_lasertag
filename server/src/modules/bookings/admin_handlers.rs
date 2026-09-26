//! Admin panel: every player's bookings.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use super::{
    dto::{AdminCreateBookingRequest, AdminListQuery, BookingResponse, UpdateBookingRequest},
    service::{self, Actor},
};
use crate::{error::AppResult, shared::pagination::Paginated, state::AppState};

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<AdminListQuery>,
) -> AppResult<Json<Paginated<BookingResponse>>> {
    let page = service::list_all(&state, query).await?;
    let tz = state.config.bookeo.timezone;
    Ok(Json(
        page.map(|r| BookingResponse::from_row_with_player(r, tz)),
    ))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BookingResponse>> {
    service::get(&state, id, None).await?;
    respond(&state, id).await
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<AdminCreateBookingRequest>,
) -> AppResult<(StatusCode, Json<BookingResponse>)> {
    let row = service::create(&state, req.player_id, req.booking, Actor::Admin).await?;
    Ok((StatusCode::CREATED, respond(&state, row.id).await?))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateBookingRequest>,
) -> AppResult<Json<BookingResponse>> {
    service::update(&state, id, req, Actor::Admin).await?;
    respond(&state, id).await
}

/// Cancels the booking (it stays listed with status `canceled`).
pub async fn cancel(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BookingResponse>> {
    service::cancel(&state, id, Actor::Admin).await?;
    respond(&state, id).await
}

async fn respond(state: &AppState, id: Uuid) -> AppResult<Json<BookingResponse>> {
    let row = service::with_player(state, id).await?;
    Ok(Json(BookingResponse::from_row_with_player(
        row,
        state.config.bookeo.timezone,
    )))
}
