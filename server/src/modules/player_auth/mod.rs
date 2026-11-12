//! Player authentication for the mobile app: sign-up and playername + password → Bearer token.
//! Player tokens carry `kind: player`, so they can't be used on admin routes (and vice versa).

mod account;
mod extractor;
mod handlers;
mod reset;

use axum::{
    Router,
    middleware::{self, FromExtractorLayer},
    routing::{get, post},
};

pub use extractor::{AuthPlayer, authenticate_player};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(handlers::login))
        .route("/signup", post(handlers::signup))
        .route("/playername-available", get(handlers::playername_available))
        .route(
            "/me",
            get(handlers::me)
                .patch(account::update_me)
                .delete(handlers::delete_me),
        )
        .route("/email/send-code", post(account::send_email_code))
        .route("/email/verify", post(account::verify_email))
        .route("/change-password", post(account::change_password))
        .route("/delete-account", post(account::request_deletion))
        .route("/delete-account/confirm", post(account::confirm_deletion))
        .route("/forgot-password", post(reset::forgot_password))
        .route("/reset-password", post(reset::reset_password))
}

/// Layer that rejects requests without a valid player token. Apply with `.route_layer(...)`.
pub fn require_player(state: &AppState) -> FromExtractorLayer<AuthPlayer, AppState> {
    middleware::from_extractor_with_state(state.clone())
}
