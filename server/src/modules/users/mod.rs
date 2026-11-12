//! Users: staff accounts for the admin panel (sign-in lives in `modules::auth`).

mod handlers;
pub mod model;
pub mod repository;

pub use model::{PublicUser, User};

use axum::{
    Router,
    routing::{get, patch, post, put},
};

use crate::state::AppState;

/// Admin-only routes; mounted at `/users` behind `auth::require_auth` in `modules::router`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list).post(handlers::create))
        .route("/me", patch(handlers::update_me))
        .route("/me/password", post(handlers::change_my_password))
        .route("/{id}", patch(handlers::update).delete(handlers::delete))
        .route("/{id}/password", put(handlers::set_password))
}
