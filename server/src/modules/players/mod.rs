//! Players: admin CRUD over player accounts. Players sign in to the mobile app via
//! `modules::player_auth`.

mod dto;
mod handlers;
pub mod model;
pub mod repository;
mod service;

pub use dto::SignupInput;
pub use model::Player;
pub use service::{check_playername, signup};

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
