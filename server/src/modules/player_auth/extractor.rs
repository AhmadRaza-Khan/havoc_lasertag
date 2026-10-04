use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use crate::{
    error::AppError,
    modules::players::repository as players,
    shared::jwt::{self, TokenKind},
    state::AppState,
};

/// Authenticated player. Only `Authorization: Bearer <token>` is accepted (the mobile app
/// stores the token); cookies are ignored so browser sessions can't be used here.
#[derive(Debug, Clone)]
pub struct AuthPlayer {
    pub id: Uuid,
}

impl FromRequestParts<AppState> for AuthPlayer {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Already verified by the route-group layer for this request.
        if let Some(player) = parts.extensions.get::<AuthPlayer>() {
            return Ok(player.clone());
        }

        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))?;

        let player = AuthPlayer {
            id: authenticate_player(token, state).await?,
        };
        parts.extensions.insert(player.clone());
        Ok(player)
    }
}

/// Validates a player token and returns the player id: must be a player token, for a player
/// that still exists and hasn't had their password changed since. Also used by the chat
/// WebSocket, which may receive the token as a query parameter.
pub async fn authenticate_player(token: &str, state: &AppState) -> Result<Uuid, AppError> {
    let claims = jwt::verify_token(token, &state.config.jwt_secret)?;
    if claims.kind != TokenKind::Player {
        return Err(AppError::Forbidden("Player access required".into()));
    }

    let changed_at = players::password_changed_at(&state.db, claims.sub)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Account no longer exists".into()))?;
    if !claims.issued_after(changed_at) {
        return Err(AppError::Unauthorized(
            "Session expired, please sign in again".into(),
        ));
    }
    Ok(claims.sub)
}
