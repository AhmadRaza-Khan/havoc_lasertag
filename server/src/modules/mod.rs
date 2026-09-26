//! Feature modules. Each module owns its routes, handlers, business logic and data access,
//! and exposes a `routes()` function that is mounted here.

pub mod auth;
pub mod health;
pub mod players;
pub mod users;

use axum::Router;

use crate::state::AppState;

/// All API routes, mounted under `/api` by [`crate::app::build`].
pub fn router(state: &AppState) -> Router<AppState> {
    let public = Router::new()
        .nest("/health", health::routes())
        .nest("/auth", auth::routes());

    // Admin-only feature routes: everything here requires a valid admin token.
    let admin = Router::new()
        .nest("/players", players::routes())
        .route_layer(auth::require_auth(state));

    public.merge(admin)
}
