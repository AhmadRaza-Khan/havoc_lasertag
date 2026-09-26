//! Private one-to-one chat between players (mobile app only).
//!
//! - REST (`routes`) loads players, conversations and history, and can send/mark read.
//! - The WebSocket (`ws_routes`) delivers messages, typing, read receipts and presence live.
//! - `hub` tracks live connections in memory; messages persist in Postgres.

mod dto;
mod events;
mod handlers;
pub mod hub;
mod model;
mod repository;
mod service;
mod ws;

use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

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
}

/// The WebSocket authenticates itself (header or `?token=`), so it's mounted outside the
/// player auth layer.
pub fn ws_routes() -> Router<AppState> {
    Router::new().route("/ws", get(ws::connect))
}
