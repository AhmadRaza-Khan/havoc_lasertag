//! Player authentication for the mobile app: sign-up and playername + password → Bearer token.
//! Player tokens carry `kind: player`, so they can't be used on admin routes (and vice versa).

mod extractor;
mod handlers;

use axum::{
    Router,
    middleware::{self, FromExtractorLayer},
    routing::{get, post},
};

pub use extractor::{AuthPlayer, authenticate_player};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(handlers::login))
        .route("/signup", post(handlers::signup))
        .route("/playername-available", get(handlers::playername_available))
        .route("/me", get(handlers::me))
}

/// Layer that rejects requests without a valid player token. Apply with `.route_layer(...)`.
pub fn require_player(state: &AppState) -> FromExtractorLayer<AuthPlayer, AppState> {
    middleware::from_extractor_with_state(state.clone())
}
