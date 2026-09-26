use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use super::{
    AUTH_COOKIE,
    jwt::{self, TokenKind},
};
use crate::{error::AppError, state::AppState};

/// Authenticated admin user extracted from the request. Add it as a handler argument to get the
/// caller, or protect a whole router group with [`super::require_auth`].
///
/// Accepts either an `Authorization: Bearer <token>` header (mobile / API clients)
/// or the httpOnly auth cookie (web client).
pub struct AuthUser {
    pub id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let bearer = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::to_owned);

        let token = match bearer {
            Some(t) => t,
            None => CookieJar::from_headers(&parts.headers)
                .get(AUTH_COOKIE)
                .map(|c| c.value().to_owned())
                .ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))?,
        };

        let claims = jwt::verify_token(&token, &state.config.jwt_secret)?;
        if claims.kind != TokenKind::Admin {
            return Err(AppError::Forbidden("Admin access required".into()));
        }
        Ok(AuthUser { id: claims.sub })
    }
}
