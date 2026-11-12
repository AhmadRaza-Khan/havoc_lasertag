//! Authentication: signup, login/logout, and the `AuthUser` extractor used to protect routes.

mod dto;
mod extractor;
mod handlers;
mod reset;
mod service;

use axum::{
    Router,
    middleware::{self, FromExtractorLayer},
    routing::{get, post},
};

pub use extractor::AuthUser;

use crate::state::AppState;

pub const AUTH_COOKIE: &str = "havoc_token";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/signup", post(handlers::signup))
        .route("/login", post(handlers::login))
        .route("/logout", post(handlers::logout))
        .route("/me", get(handlers::me))
        .route("/forgot-password", post(reset::forgot_password))
        .route("/reset-password", post(reset::reset_password))
}

/// A fresh session cookie for `user_id`, e.g. after they changed their own password (which ends
/// every earlier session, this browser's included).
pub fn new_session_cookie(
    state: &AppState,
    user_id: uuid::Uuid,
) -> crate::error::AppResult<axum_extra::extract::cookie::Cookie<'static>> {
    let token = crate::shared::jwt::create_token(
        user_id,
        crate::shared::jwt::TokenKind::Admin,
        &state.config.jwt_secret,
        state.config.jwt_ttl_hours,
    )?;
    Ok(handlers::session_cookie(state, token))
}

/// Layer that rejects requests without a valid admin token. Apply with `.route_layer(...)`.
pub fn require_auth(state: &AppState) -> FromExtractorLayer<AuthUser, AppState> {
    middleware::from_extractor_with_state(state.clone())
}
