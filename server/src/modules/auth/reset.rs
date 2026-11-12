//! "Forgot password" for staff: a 6-digit code emailed to the staff member's address (set in
//! Settings). Accounts without an email are reset by the owner by hand.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;

use crate::{
    error::{AppError, AppResult},
    modules::users::repository as users,
    shared::{
        email_code::{self, Purpose},
        login_throttle::Attempt,
        password,
        validation::validate_password,
    },
    state::AppState,
};

#[derive(Deserialize)]
pub struct ForgotRequest {
    /// Username or email address.
    pub login: String,
}

#[derive(Deserialize)]
pub struct ResetRequest {
    pub login: String,
    pub code: String,
    pub new_password: String,
}

/// `POST /api/auth/forgot-password`: emails a code if the account exists and has an email.
/// Always 204, so it can't be used to find out who has an account.
pub async fn forgot_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ForgotRequest>,
) -> AppResult<StatusCode> {
    let attempt = Attempt::per_client("staff-reset-request", &headers);
    state.login_throttle.check(&attempt)?;
    state.login_throttle.record_failure(&attempt);
    if req.login.trim().is_empty() {
        return Err(AppError::BadRequest("Enter your username or email".into()));
    }
    let Some(user) = users::find_by_login(&state.db, &req.login).await? else {
        return Ok(StatusCode::NO_CONTENT);
    };
    let Some(email) = user.email.clone() else {
        tracing::info!(user_id = %user.id, "staff password reset asked for an account without email");
        return Ok(StatusCode::NO_CONTENT);
    };
    if let Some(code) = email_code::issue(&state.db, user.id, Purpose::StaffReset).await? {
        state.mailer.send_later(email_code::email(
            &email,
            &user.name,
            "Your Havoc admin password reset code",
            "Your Havoc admin password reset code is",
            &code,
        ));
        tracing::info!(user_id = %user.id, "staff password reset code sent");
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/auth/reset-password`: sets a new password with the emailed code. Every existing
/// session of that account ends.
pub async fn reset_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ResetRequest>,
) -> AppResult<StatusCode> {
    let invalid =
        || AppError::BadRequest("That code is wrong or has expired. Ask for a new one.".into());
    let attempt = Attempt::new("staff-reset", &req.login.trim().to_lowercase(), &headers);
    state.login_throttle.check(&attempt)?;
    validate_password(&req.new_password)?;
    let Some(user) = users::find_by_login(&state.db, &req.login).await? else {
        state.login_throttle.record_failure(&attempt);
        password::verify_or_dummy(req.code, None).await?;
        return Err(invalid());
    };
    if !email_code::check(&state.db, user.id, Purpose::StaffReset, &req.code).await? {
        state.login_throttle.record_failure(&attempt);
        return Err(invalid());
    }
    let hash = password::hash(req.new_password).await?;
    users::set_password(&state.db, user.id, &hash).await?;
    state.login_throttle.record_success(&attempt);
    tracing::info!(user_id = %user.id, "staff password reset with emailed code");
    Ok(StatusCode::NO_CONTENT)
}
