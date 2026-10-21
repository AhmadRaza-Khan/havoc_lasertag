//! The website's leaderboard (WordPress plugin): read-only, authenticated with the site key
//! (`X-Api-Key`). Players appear by playername only — the same public view as the app.

use axum::{
    Json,
    extract::{FromRequestParts, Path, State},
    http::request::Parts,
};
use serde::Serialize;
use uuid::Uuid;

use super::{
    dto::{PublicPlayerStats, PublicStanding},
    repository, service,
};
use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

/// Rows the website shows.
const LEADERBOARD_SIZE: usize = 30;

/// A request carrying the website's API key. Without a configured key the API doesn't exist.
pub struct WebsiteKey;

impl FromRequestParts<AppState> for WebsiteKey {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(expected) = state.config.website_api_key.as_deref() else {
            return Err(AppError::NotFound("Not found".into()));
        };
        let given = parts
            .headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if !constant_time_eq(given.as_bytes(), expected.as_bytes()) {
            return Err(AppError::Unauthorized("Invalid API key".into()));
        }
        Ok(Self)
    }
}

/// Compares without leaking how much of the key matched through timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Serialize)]
pub struct WebsiteLeaderboard {
    /// The top 30, best first.
    pub standings: Vec<PublicStanding>,
    /// Everyone who has played at least one game.
    pub total_ranked: i64,
}

/// `GET /api/website/leaderboard`
pub async fn leaderboard(
    State(state): State<AppState>,
    _key: WebsiteKey,
) -> AppResult<Json<WebsiteLeaderboard>> {
    let tz = state.config.bookeo.timezone;
    let ranked: Vec<PublicStanding> = service::leaderboard(&state)
        .await?
        .into_iter()
        .filter_map(|r| PublicStanding::from_row(r, tz, Uuid::nil()))
        .collect();
    Ok(Json(WebsiteLeaderboard {
        total_ranked: ranked.len() as i64,
        standings: ranked.into_iter().take(LEADERBOARD_SIZE).collect(),
    }))
}

/// `GET /api/website/players/{id}`: a ranked player's totals and personal stats.
pub async fn player(
    State(state): State<AppState>,
    _key: WebsiteKey,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PublicPlayerStats>> {
    let row = repository::standing(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Player not found".into()))?;
    let standing = PublicStanding::from_row(row, state.config.bookeo.timezone, Uuid::nil())
        .ok_or_else(|| AppError::NotFound("This player hasn't played yet".into()))?;
    Ok(Json(PublicPlayerStats {
        standing,
        details: service::player_details(&state, id).await?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_keys() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(!constant_time_eq(b"", b"abc"));
    }
}
