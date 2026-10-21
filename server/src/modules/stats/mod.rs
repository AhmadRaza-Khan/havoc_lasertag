//! Game statistics from the arena's PDF reports.
//!
//! Admins upload a report; it's read (page 1 text + OCR of each player's page), saved as one
//! game with a record per player, and the file is deleted. The admin dashboard shows the
//! general report (all players by total points); each player reads only their own records.

mod admin_handlers;
mod dto;
pub mod import;
mod model;
mod player_handlers;
mod repository;
mod service;
mod website_handlers;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};

pub use service::{link_unclaimed_records, recover};

use crate::state::AppState;

/// The website's read-only leaderboard (WordPress plugin); authenticated by the site key, not a
/// login. Mounted at `/website`.
pub fn website_routes() -> Router<AppState> {
    Router::new()
        .route("/leaderboard", get(website_handlers::leaderboard))
        .route("/players/{id}", get(website_handlers::player))
}

/// Admin panel routes; mounted behind `auth::require_auth`.
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/imports",
            post(admin_handlers::upload)
                .get(admin_handlers::list_imports)
                // multipart overhead on top of the file itself
                .layer(DefaultBodyLimit::max(service::MAX_UPLOAD_BYTES + 64 * 1024)),
        )
        .route("/imports/{id}", get(admin_handlers::get_import))
        .route("/leaderboard", get(admin_handlers::leaderboard))
        .route("/players/{id}", get(admin_handlers::player_standing))
        .route("/players/{id}/games", get(admin_handlers::player_games))
}

/// Mobile app routes; mounted behind `player_auth::require_player`.
pub fn player_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(player_handlers::summary))
        .route("/leaderboard", get(player_handlers::leaderboard))
        .route("/games", get(player_handlers::games))
        .route("/games/{game_id}", get(player_handlers::game))
        .route("/players/{id}", get(player_handlers::other_summary))
        .route("/players/{id}/games", get(player_handlers::other_games))
        .route(
            "/players/{id}/games/{game_id}",
            get(player_handlers::other_game),
        )
}
