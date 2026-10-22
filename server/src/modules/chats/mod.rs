//! Private one-to-one chat between players (mobile app), and staff chatting with players
//! as the "Havoc Admin" player from the admin panel (`admin_routes`).
//!
//! - REST (`routes`) loads players, conversations and history, and can send/mark read.
//! - The WebSocket (`ws_routes`) delivers messages, typing, read receipts and presence live.
//! - `hub` tracks live connections in memory; messages persist in Postgres.

mod admin_handlers;
mod dto;
mod events;
mod handlers;
pub mod hub;
mod model;
mod repository;
mod safety;
mod service;
mod ws;

use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

pub use service::disconnect_player;

/// Player-only REST routes; mounted behind `player_auth::require_player`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/players", get(handlers::players))
        .route("/players/{id}", get(handlers::player))
        .route("/conversations", get(handlers::conversations))
        .route(
            "/conversations/{player_id}/messages",
            get(handlers::messages).post(handlers::send),
        )
        .route("/conversations/{player_id}/read", post(handlers::read))
        .route(
            "/players/{id}/block",
            post(safety::block).delete(safety::unblock),
        )
        .route("/blocks", get(safety::blocks))
        .route("/reports", post(safety::report))
}

/// Admin panel routes (chat as "Havoc Admin"); mounted at `/chats` behind `auth::require_auth`.
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/players", get(admin_handlers::players))
        .route("/conversations", get(admin_handlers::conversations))
        .route(
            "/conversations/{player_id}/messages",
            get(admin_handlers::messages).post(admin_handlers::send),
        )
        .route(
            "/conversations/{player_id}/read",
            post(admin_handlers::read),
        )
        .route("/reports", get(safety::admin_reports))
        .route("/reports/{id}/resolve", post(safety::admin_resolve))
}

/// The WebSocket authenticates itself (header or `?token=`), so it's mounted outside the
/// player auth layer.
pub fn ws_routes() -> Router<AppState> {
    Router::new().route("/ws", get(ws::connect))
}
