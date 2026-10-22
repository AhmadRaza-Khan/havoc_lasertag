//! Players: admin CRUD over player accounts. Players sign in to the mobile app via
//! `modules::player_auth`.

mod dto;
mod handlers;
pub mod model;
pub mod repository;
mod service;

pub use dto::SignupInput;
pub use model::Player;
pub use service::{check_playername, delete as delete_player, signup};

use axum::{Router, routing::get};
use uuid::Uuid;

use crate::state::AppState;

/// The "Havoc Admin" player (migration 0013): staff chat with players as this account from
/// the admin panel. It can't sign in and isn't listed with the other players.
pub const HAVOC_ADMIN_ID: Uuid = Uuid::from_u128(0xad01);

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
