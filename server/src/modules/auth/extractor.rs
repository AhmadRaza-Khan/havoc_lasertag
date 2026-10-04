use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use super::AUTH_COOKIE;
use crate::{
    error::AppError,
    modules::users::repository as users,
    shared::jwt::{self, TokenKind},
    state::AppState,
};

/// Authenticated admin user extracted from the request. Add it as a handler argument to get the
/// caller, or protect a whole router group with [`super::require_auth`].
///
/// Accepts either an `Authorization: Bearer <token>` header or the httpOnly auth cookie (web
/// client). The token must be an admin token, and the account must still exist with no
/// password change since the token was issued.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Already verified by the route-group layer for this request.
        if let Some(user) = parts.extensions.get::<AuthUser>() {
            return Ok(user.clone());
        }

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

        let changed_at = users::password_changed_at(&state.db, claims.sub)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Account no longer exists".into()))?;
        if !claims.issued_after(changed_at) {
            return Err(AppError::Unauthorized(
                "Session expired, please sign in again".into(),
            ));
        }

        let user = AuthUser { id: claims.sub };
        parts.extensions.insert(user.clone());
        Ok(user)
    }
}
