//! Players: admin CRUD over player accounts (players will later sign in from the mobile app).

mod dto;
mod handlers;
mod model;
mod repository;
mod service;

use axum::{Router, routing::get};

use crate::state::AppState;

/// Admin-only routes; mounted behind `auth::require_auth` in `modules::router`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list).post(handlers::create))
        .route(
            "/{id}",
            get(handlers::get)
                .put(handlers::update)
                .delete(handlers::delete),
        )
}
