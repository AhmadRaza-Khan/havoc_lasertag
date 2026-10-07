use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;

use super::service;
use crate::{error::AppResult, modules::player_auth::AuthPlayer, state::AppState};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// Expo push token, e.g. `ExponentPushToken[…]`.
    pub token: String,
    /// `ios` or `android`.
    pub platform: String,
}

#[derive(Debug, Deserialize)]
pub struct UnregisterRequest {
    pub token: String,
}

/// After sign-in: this phone should get the player's notifications.
pub async fn register(
    State(state): State<AppState>,
    player: AuthPlayer,
    Json(req): Json<RegisterRequest>,
) -> AppResult<StatusCode> {
    service::register(&state, player.id, req.token.trim(), req.platform.trim()).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// On sign-out: stop notifications to this phone.
pub async fn unregister(
    State(state): State<AppState>,
    player: AuthPlayer,
    Json(req): Json<UnregisterRequest>,
) -> AppResult<StatusCode> {
    service::unregister(&state, player.id, req.token.trim()).await?;
    Ok(StatusCode::NO_CONTENT)
}
