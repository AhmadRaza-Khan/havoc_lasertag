//! Authentication: signup, login/logout, and the `AuthUser` extractor used to protect routes.

mod dto;
mod extractor;
mod handlers;
mod jwt;
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
}

/// Layer that rejects requests without a valid admin token. Apply with `.route_layer(...)`.
pub fn require_auth(state: &AppState) -> FromExtractorLayer<AuthUser, AppState> {
    middleware::from_extractor_with_state(state.clone())
}
