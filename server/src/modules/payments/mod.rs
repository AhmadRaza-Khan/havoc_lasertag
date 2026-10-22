//! Online payment of app bookings through SkipCash's hosted payment page.
//!
//! Flow: the app books → `POST /player/bookings/{id}/payment` returns SkipCash's page → the
//! app opens it in a WebView and polls `GET .../payment` → when SkipCash reports the payment
//! paid, the booking is marked paid and the payment is recorded in Bookeo. Leaving the page
//! unpaid (`DELETE .../payment`) cancels a "pay now" booking at once. SkipCash's signed webhook
//! (`POST /api/webhooks/skipcash`) and the `worker` catch payments finished with the app
//! closed; the worker also cancels bookings not paid in time (admins choose "pay now" or
//! "pay later" in Settings).

mod credentials;
mod emails;
mod handlers;
mod model;
mod repository;
mod service;
pub mod worker;

pub use credentials::load as load_skipcash;
pub use handlers::skipcash_webhook;
pub use model::PaymentMode;
pub use service::on_booking_created;

use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

/// "Pay now" bookings are canceled as soon as the customer leaves the payment page unpaid
/// (`DELETE .../payment`). This is the backstop for when the app never says so (killed,
/// offline): long enough to finish 3-D Secure on the payment page.
pub const PAY_NOW_MINUTES: i64 = 10;

/// Admin-only routes; mounted at `/settings` behind `auth::require_auth`.
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/payments",
            get(handlers::get_settings).put(handlers::put_settings),
        )
        .route(
            "/skipcash",
            get(handlers::get_skipcash)
                .put(handlers::put_skipcash)
                .delete(handlers::delete_skipcash),
        )
        .route("/skipcash/test", post(handlers::test_skipcash))
}

/// Mobile app routes, merged into `/player/bookings`.
pub fn player_booking_routes() -> Router<AppState> {
    Router::new().route(
        "/{id}/payment",
        get(handlers::status)
            .post(handlers::start)
            .delete(handlers::abandon),
    )
}
