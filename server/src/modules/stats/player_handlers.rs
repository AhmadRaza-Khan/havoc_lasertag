//! Mobile app: a player's own statistics. Every query is scoped to the authenticated player.

use axum::{
    Json,
    extract::{Path, Query, State},
};
use uuid::Uuid;

use super::{
    dto::{GameRecord, Leaderboard, MyStats, PageQuery, PublicStanding, Standing},
    repository, service,
};
use crate::{
    error::AppResult,
    modules::player_auth::AuthPlayer,
    shared::pagination::{PageParams, Paginated},
    state::AppState,
};

/// All-time totals and leaderboard position.
pub async fn summary(
    State(state): State<AppState>,
    player: AuthPlayer,
) -> AppResult<Json<MyStats>> {
    let standing = service::player_standing(&state, player.id).await?;
    let total_players = repository::player_count(&state.db).await?;
    Ok(Json(MyStats {
        standing: Standing::from_row(standing, state.config.bookeo.timezone),
        total_players,
    }))
}

/// Rows shown in the app's leaderboard.
const LEADERBOARD_SIZE: usize = 100;

/// The general leaderboard: every player who has played, ranked by all-time points.
/// Any signed-in player can see it; others appear by playername only.
pub async fn leaderboard(
    State(state): State<AppState>,
    player: AuthPlayer,
) -> AppResult<Json<Leaderboard>> {
    let tz = state.config.bookeo.timezone;
    let ranked: Vec<PublicStanding> = service::leaderboard(&state)
        .await?
        .into_iter()
        .filter_map(|r| PublicStanding::from_row(r, tz, player.id))
        .collect();
    let total_ranked = ranked.len() as i64;
    let me = ranked.iter().find(|s| s.is_me).cloned();
    Ok(Json(Leaderboard {
        standings: ranked.into_iter().take(LEADERBOARD_SIZE).collect(),
        me,
        total_ranked,
    }))
}

/// The player's game records, newest first.
pub async fn games(
    State(state): State<AppState>,
    player: AuthPlayer,
    Query(q): Query<PageQuery>,
) -> AppResult<Json<Paginated<GameRecord>>> {
    let params = PageParams {
        page: q.page,
        per_page: q.per_page,
    };
    let tz = state.config.bookeo.timezone;
    let page = service::player_records(&state, player.id, params).await?;
    Ok(Json(page.map(|r| GameRecord::from_row(r, tz))))
}

/// The player's record in one game (404 if they didn't play it).
pub async fn game(
    State(state): State<AppState>,
    player: AuthPlayer,
    Path(game_id): Path<Uuid>,
) -> AppResult<Json<GameRecord>> {
    let row = service::player_record(&state, player.id, game_id).await?;
    Ok(Json(GameRecord::from_row(
        row,
        state.config.bookeo.timezone,
    )))
}
