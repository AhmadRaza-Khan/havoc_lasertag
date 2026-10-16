//! Offers: promotions as text, an image, or both. Admins manage them; the mobile app reads the
//! ones live today. Images are stored in Postgres and served at a versioned public URL.

mod dto;
mod handlers;
mod model;
mod repository;
mod service;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, put},
};

use crate::state::AppState;

/// Admin-only routes; mounted behind `auth::require_auth` in `modules::router`.
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list).post(handlers::create))
        .route(
            "/{id}",
            get(handlers::get)
                .put(handlers::update)
                .delete(handlers::delete),
        )
        .route(
            "/{id}/image",
            put(handlers::upload_image)
                .delete(handlers::remove_image)
                .layer(DefaultBodyLimit::max(service::MAX_IMAGE_BYTES + 64 * 1024)),
        )
}

/// Public (no token): app `<Image>` requests can't always carry headers. Mounted at
/// `/offer-images`.
pub fn public_routes() -> Router<AppState> {
    Router::new().route("/{id}", get(handlers::serve_image))
}

/// Mobile app routes; mounted behind `player_auth::require_player` in `modules::router`.
pub fn player_routes() -> Router<AppState> {
    Router::new().route("/", get(handlers::live))
}
