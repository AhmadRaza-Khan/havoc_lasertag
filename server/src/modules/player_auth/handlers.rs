use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use super::AuthPlayer;
use crate::{
    error::{AppError, AppResult},
    modules::players::{Player, repository as players},
    shared::{
        jwt::{self, TokenKind},
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
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let invalid = || AppError::Unauthorized("Invalid playername or password".into());

    let (id, hash) = players::find_credentials(&state.db, &normalize_handle(&req.playername))
        .await?
        .ok_or_else(invalid)?;

    if !password::verify(req.password, hash).await? {
        return Err(invalid());
    }

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
