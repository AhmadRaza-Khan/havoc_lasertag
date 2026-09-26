//! Bookings of Bookeo slots, linked to players. Players manage their own bookings from the
//! mobile app; admins manage everyone's from the web panel. Both go through `service`.

mod admin_handlers;
mod dto;
mod model;
mod player_handlers;
mod repository;
mod service;

use axum::{Router, routing::get};

use crate::state::AppState;

/// Mobile app routes; mounted behind `player_auth::require_player`.
pub fn player_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(player_handlers::list).post(player_handlers::create),
        )
        .route("/{id}", get(player_handlers::get))
}

/// Admin panel routes; mounted behind `auth::require_auth`.
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(admin_handlers::list).post(admin_handlers::create))
        .route(
            "/{id}",
            get(admin_handlers::get)
                .put(admin_handlers::update)
                .delete(admin_handlers::cancel),
        )
}
