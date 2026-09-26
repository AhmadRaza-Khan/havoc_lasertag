use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use crate::{
    error::AppError,
    shared::jwt::{self, TokenKind},
    state::AppState,
};

/// Authenticated player. Only `Authorization: Bearer <token>` is accepted (the mobile app
/// stores the token); cookies are ignored so browser sessions can't be used here.
pub struct AuthPlayer {
    pub id: Uuid,
}

impl FromRequestParts<AppState> for AuthPlayer {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))?;

        let claims = jwt::verify_token(token, &state.config.jwt_secret)?;
        if claims.kind != TokenKind::Player {
            return Err(AppError::Forbidden("Player access required".into()));
        }
        Ok(AuthPlayer { id: claims.sub })
    }
}
