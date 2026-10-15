//! Player profile pictures: uploaded from the app (resized on the phone), stored in Postgres,
//! served at a versioned URL (`/api/avatars/{player_id}?v=…`) so they can be cached for good.

mod handlers;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, put},
};

use crate::state::AppState;

/// Largest accepted picture. The app sends ~1024 px JPEGs (about 100–250 KB); nginx caps
/// request bodies at 1 MB.
pub const MAX_BYTES: usize = 900 * 1024;

/// Public (no token): image requests from `<Image>` components can't always carry headers,
/// and the URL is only known to signed-in players.
pub fn public_routes() -> Router<AppState> {
    Router::new().route("/{player_id}", get(handlers::serve))
}

/// Mobile app: the signed-in player's own picture. Mounted at `/player/me/avatar`.
pub fn player_routes() -> Router<AppState> {
    Router::new().route(
        "/",
        put(handlers::upload)
            .delete(handlers::remove)
            .layer(DefaultBodyLimit::max(MAX_BYTES + 64 * 1024)),
    )
}
