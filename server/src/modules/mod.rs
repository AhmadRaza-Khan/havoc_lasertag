//! Feature modules. Each module owns its routes, handlers, business logic and data access,
//! and exposes a `routes()` function that is mounted here.

pub mod auth;
pub mod avatars;
pub mod bookings;
pub mod chats;
pub mod games;
pub mod health;
pub mod offers;
pub mod payments;
pub mod player_auth;
pub mod players;
pub mod push;
pub mod stats;
pub mod users;

use axum::Router;

use crate::state::AppState;

/// All API routes, mounted under `/api` by [`crate::app::build`].
pub fn router(state: &AppState) -> Router<AppState> {
    let public = Router::new()
        .nest("/health", health::routes())
        .nest("/avatars", avatars::public_routes())
        .nest("/auth", auth::routes())
        .nest("/player/auth", player_auth::routes())
        // Authenticates itself: WebSocket clients may pass the token as a query parameter.
        .nest("/player/chats", chats::ws_routes());

    // Admin panel (web): everything here requires an admin token.
    let admin = Router::new()
        .nest("/players", players::routes())
        .nest("/games", games::routes())
        .nest("/bookings", bookings::admin_routes())
        .nest("/stats", stats::admin_routes())
        .nest("/offers", offers::admin_routes())
        .nest("/settings", payments::admin_routes())
        .route_layer(auth::require_auth(state));

    // Mobile app: everything here requires a player token.
    let player = Router::new()
        .nest("/player/games", games::routes())
        .nest("/player/bookings", bookings::player_routes())
        .nest("/player/chats", chats::routes())
        .nest("/player/stats", stats::player_routes())
        .nest("/player/offers", offers::player_routes())
        .nest("/player/push-tokens", push::player_routes())
        .nest("/player/me/avatar", avatars::player_routes())
        .route_layer(player_auth::require_player(state));

    public.merge(admin).merge(player)
}
