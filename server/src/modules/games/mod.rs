//! Games (Bookeo products) and their available slots. The same read-only routes are mounted
//! for players (`/api/player/games`) and admins (`/api/games`).

pub mod dto;
mod handlers;
mod range;
pub mod service;

use axum::{Router, routing::get};

use crate::state::AppState;

/// Mounted in both the player and admin groups in `modules::router`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list))
        .route("/{product_id}/slots", get(handlers::slots))
}
