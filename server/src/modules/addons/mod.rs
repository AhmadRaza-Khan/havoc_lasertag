//! Add-on pictures: admins upload one per add-on (Bookeo "option") in Settings; the booking
//! screens show them next to the add-ons. Prices stay Bookeo's: it adds them to the booking.
//! Pictures are stored in Postgres and served at a versioned public URL.

mod handlers;
mod service;

pub use service::{addon_key, image_urls};

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, put},
};

use crate::state::AppState;

/// Admin-only routes, merged into `/settings` behind `auth::require_auth` in `modules::router`.
pub fn admin_routes() -> Router<AppState> {
    Router::new().route("/addons", get(handlers::list)).route(
        "/addons/{key}/image",
        put(handlers::upload_image)
            .delete(handlers::remove_image)
            .layer(DefaultBodyLimit::max(service::MAX_IMAGE_BYTES + 64 * 1024)),
    )
}

/// Public (no token): app `<Image>` requests can't always carry headers. Mounted at
/// `/addon-images`.
pub fn public_routes() -> Router<AppState> {
    Router::new().route("/{key}", get(handlers::serve_image))
}
