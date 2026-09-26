//! Mobile app: a player's own bookings.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use super::{
    dto::{BookingResponse, CreateBookingRequest, ListQuery},
    service::{self, Actor},
};
use crate::{
    error::AppResult, modules::player_auth::AuthPlayer, shared::pagination::Paginated,
    state::AppState,
};

pub async fn create(
    State(state): State<AppState>,
    player: AuthPlayer,
    Json(req): Json<CreateBookingRequest>,
) -> AppResult<(StatusCode, Json<BookingResponse>)> {
    let row = service::create(&state, player.id, req, Actor::Player).await?;
    let tz = state.config.bookeo.timezone;
    Ok((
        StatusCode::CREATED,
        Json(BookingResponse::from_row(row, tz)),
    ))
}

pub async fn list(
    State(state): State<AppState>,
    player: AuthPlayer,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Paginated<BookingResponse>>> {
    let page = service::list_for_player(&state, player.id, query).await?;
    let tz = state.config.bookeo.timezone;
    Ok(Json(page.map(|r| BookingResponse::from_row(r, tz))))
}

pub async fn get(
    State(state): State<AppState>,
    player: AuthPlayer,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BookingResponse>> {
    let row = service::get(&state, id, Some(player.id)).await?;
    Ok(Json(BookingResponse::from_row(
        row,
        state.config.bookeo.timezone,
    )))
}
