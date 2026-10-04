use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};

use super::AuthPlayer;
use crate::{
    error::{AppError, AppResult},
    modules::players::{Player, repository as players},
    shared::{
        jwt::{self, TokenKind},
        login_throttle::Attempt,
        password,
        validation::normalize_handle,
    },
    state::AppState,
};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub playername: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub player: Player,
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let invalid = || AppError::Unauthorized("Invalid playername or password".into());
    let playername = normalize_handle(&req.playername);
    let attempt = Attempt::new("player", &playername, &headers);
    state.login_throttle.check(&attempt)?;

    let credentials = players::find_credentials(&state.db, &playername).await?;
    // Same cost whether or not the account exists (no playername enumeration by timing).
    let hash = credentials.as_ref().map(|(_, h)| h.clone());
    if !password::verify_or_dummy(req.password, hash).await? {
        state.login_throttle.record_failure(&attempt);
        return Err(invalid());
    }
    let (id, _) = credentials.ok_or_else(invalid)?;
    state.login_throttle.record_success(&attempt);

    let token = jwt::create_token(
        id,
        TokenKind::Player,
        &state.config.jwt_secret,
        state.config.jwt_ttl_hours,
    )?;
    let player = current_player(&state, id).await?;

    Ok(Json(LoginResponse { token, player }))
}

pub async fn me(State(state): State<AppState>, auth: AuthPlayer) -> AppResult<Json<Player>> {
    Ok(Json(current_player(&state, auth.id).await?))
}

async fn current_player(state: &AppState, id: uuid::Uuid) -> AppResult<Player> {
    players::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Player no longer exists".into()))
}
