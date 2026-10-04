//! Offers: text promotions. Admins manage them; the mobile app reads the ones live today.

mod dto;
mod handlers;
mod model;
mod repository;
mod service;

use axum::{Router, routing::get};

use crate::state::AppState;

/// Admin-only routes; mounted behind `auth::require_auth` in `modules::router`.
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list).post(handlers::create))
        .route(
            "/{id}",
            get(handlers::get)
                .put(handlers::update)
                .delete(handlers::delete),
        )
}

/// Mobile app routes; mounted behind `player_auth::require_player` in `modules::router`.
pub fn player_routes() -> Router<AppState> {
    Router::new().route("/", get(handlers::live))
}
