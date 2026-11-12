//! "Forgot password" for players: `POST /forgot-password` emails a 6-digit code to the player's
//! address; `POST /reset-password` sets a new password with it. Answers never reveal whether an
//! account exists.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    modules::{chats, players::HAVOC_ADMIN_ID, players::repository as players, push},
    shared::{
        email_code::{self, Purpose},
        login_throttle::Attempt,
        password,
        validation::{normalize_handle, validate_password},
    },
    state::AppState,
};

#[derive(Deserialize)]
pub struct ForgotRequest {
    /// Playername or email address.
    pub login: String,
}

#[derive(Deserialize)]
pub struct ResetRequest {
    pub login: String,
    pub code: String,
    pub new_password: String,
}

#[derive(sqlx::FromRow)]
struct Account {
    id: Uuid,
    full_name: String,
    email: Option<String>,
}

/// `POST /api/player/auth/forgot-password`: emails a code if the account exists and has an email
/// address. Always answers 204, so it can't be used to find out who has an account.
pub async fn forgot_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ForgotRequest>,
) -> AppResult<StatusCode> {
    // Each request counts, per network: at most a few per window.
    let attempt = Attempt::per_client("reset-request", &headers);
    state.login_throttle.check(&attempt)?;
    state.login_throttle.record_failure(&attempt);

    let Some(account) = find_account(&state, &req.login).await? else {
        return Ok(StatusCode::NO_CONTENT);
    };
    let Some(email) = account.email.filter(|e| !e.trim().is_empty()) else {
        tracing::info!(player_id = %account.id, "password reset asked for an account without email");
        return Ok(StatusCode::NO_CONTENT);
    };
    if let Some(code) = email_code::issue(&state.db, account.id, Purpose::PlayerReset).await? {
        state.mailer.send_later(email_code::email(
            &email,
            &account.full_name,
            "Your Havoc password reset code",
            "Your Havoc password reset code is",
            &code,
        ));
        tracing::info!(player_id = %account.id, "password reset code sent");
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/player/auth/reset-password`: sets a new password with the emailed code. Signs out
/// every existing session (as any password change does).
pub async fn reset_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ResetRequest>,
) -> AppResult<StatusCode> {
    let invalid =
        || AppError::BadRequest("That code is wrong or has expired. Ask for a new one.".into());
    let attempt = Attempt::new("reset", &req.login.trim().to_lowercase(), &headers);
    state.login_throttle.check(&attempt)?;
    validate_password(&req.new_password)?;

    let Some(account) = find_account(&state, &req.login).await? else {
        state.login_throttle.record_failure(&attempt);
        // Same cost as a real check (no account enumeration by timing).
        password::verify_or_dummy(req.code, None).await?;
        return Err(invalid());
    };
    if !email_code::check(&state.db, account.id, Purpose::PlayerReset, &req.code).await? {
        state.login_throttle.record_failure(&attempt);
        return Err(invalid());
    }

    let hash = password::hash(req.new_password).await?;
    players::set_password(&state.db, account.id, &hash).await?;
    state.login_throttle.record_success(&attempt);

    // Old sessions are now rejected: close live chat and stop pushes to old phones.
    chats::disconnect_player(&state, account.id);
    push::forget_player(&state, account.id).await;
    tracing::info!(player_id = %account.id, "password reset with emailed code");
    Ok(StatusCode::NO_CONTENT)
}

/// A player by playername or email address (not the staff chat account).
async fn find_account(state: &AppState, login: &str) -> AppResult<Option<Account>> {
    let login = login.trim();
    if login.is_empty() {
        return Err(AppError::BadRequest(
            "Enter your playername or email".into(),
        ));
    }
    Ok(sqlx::query_as(
        "SELECT id, full_name, email FROM players
         WHERE (playername = $1 OR lower(email) = lower($2)) AND id <> $3
         LIMIT 1",
    )
    .bind(normalize_handle(login))
    .bind(login)
    .bind(HAVOC_ADMIN_ID)
    .fetch_optional(&state.db)
    .await?)
}
