//! Mobile app: a player's own bookings and the ones other players added them to.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use super::{
    dto::{BookingResponse, CreateBookingRequest, ListQuery},
    model::BookingRow,
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
    Ok((StatusCode::CREATED, Json(respond(&state, row).await?)))
}

pub async fn list(
    State(state): State<AppState>,
    player: AuthPlayer,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Paginated<BookingResponse>>> {
    let page = service::list_for_player(&state, player.id, query).await?;
    let tz = state.config.bookeo.timezone;
    let others = service::other_players_lookup(&state, &page.items).await?;
    Ok(Json(page.map(|r| {
        let (ids, owner) = (r.other_players.clone(), r.player_id);
        BookingResponse::from_row(r, tz)
            .with_other_players(&ids, &others)
            .with_player_from(owner, &others)
    })))
}

pub async fn get(
    State(state): State<AppState>,
    player: AuthPlayer,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BookingResponse>> {
    let row = service::get(&state, id, Some(player.id)).await?;
    Ok(Json(respond(&state, row).await?))
}

async fn respond(state: &AppState, row: BookingRow) -> AppResult<BookingResponse> {
    let others = service::other_players_lookup(state, [&row]).await?;
    let (ids, owner) = (row.other_players.clone(), row.player_id);
    Ok(BookingResponse::from_row(row, state.config.bookeo.timezone)
        .with_other_players(&ids, &others)
        .with_player_from(owner, &others))
}
