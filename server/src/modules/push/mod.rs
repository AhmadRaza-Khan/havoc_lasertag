//! Push notifications to players' phones (via Expo). The app registers its push token after
//! signing in; chat messages to players who don't have the app open are pushed.

mod handlers;
mod repository;
mod service;

pub use service::{forget_player, notify_chat_message};

use axum::{Router, routing::post};

use crate::state::AppState;

/// Mobile app routes; mounted at `/player/push-tokens` behind `player_auth::require_player`.
pub fn player_routes() -> Router<AppState> {
    Router::new().route("/", post(handlers::register).delete(handlers::unregister))
}
