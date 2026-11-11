//! "Forgot password" for players: `POST /forgot-password` emails a 6-digit code to the player's
//! address; `POST /reset-password` sets a new password with it. Answers never reveal whether an
//! account exists. Staff accounts have no email, so this is for players only.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    integrations::email::Email,
    modules::{chats, players::HAVOC_ADMIN_ID, push},
    shared::{
        login_throttle::Attempt,
        password,
        validation::{normalize_handle, validate_password},
    },
    state::AppState,
};

/// How long a code works.
const CODE_TTL_MINUTES: i64 = 15;
/// Wrong tries before the code stops working.
const MAX_ATTEMPTS: i32 = 5;
/// A new code isn't emailed again within this many seconds (no email flooding).
const RESEND_AFTER_SECONDS: i64 = 60;

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
    let recent: Option<bool> = sqlx::query_scalar(
        "SELECT created_at > now() - make_interval(secs => $2) FROM player_password_resets WHERE player_id = $1",
    )
    .bind(account.id)
    .bind(RESEND_AFTER_SECONDS as f64)
    .fetch_optional(&state.db)
    .await?;
    if recent == Some(true) {
        return Ok(StatusCode::NO_CONTENT);
    }

    let code = new_code();
    let code_hash = password::hash(code.clone()).await?;
    sqlx::query(
        "INSERT INTO player_password_resets (player_id, code_hash, expires_at) VALUES ($1, $2, $3)
         ON CONFLICT (player_id) DO UPDATE
             SET code_hash = $2, expires_at = $3, attempts = 0, created_at = now()",
    )
    .bind(account.id)
    .bind(&code_hash)
    .bind(Utc::now() + Duration::minutes(CODE_TTL_MINUTES))
    .execute(&state.db)
    .await?;
    state
        .mailer
        .send_later(reset_email(&email, &account.full_name, &code));
    tracing::info!(player_id = %account.id, "password reset code sent");
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

    let account = find_account(&state, &req.login).await?;
    let pending: Option<(String, i32)> = match &account {
        Some(a) => {
            sqlx::query_as(
                "SELECT code_hash, attempts FROM player_password_resets
                 WHERE player_id = $1 AND expires_at > now()",
            )
            .bind(a.id)
            .fetch_optional(&state.db)
            .await?
        }
        None => None,
    };
    let code = req.code.trim().to_string();
    let (Some(account), Some((code_hash, attempts))) = (account, pending) else {
        state.login_throttle.record_failure(&attempt);
        // Same cost as a real check (no account enumeration by timing).
        password::verify_or_dummy(code, None).await?;
        return Err(invalid());
    };
    if attempts >= MAX_ATTEMPTS || !password::verify(code, code_hash).await? {
        state.login_throttle.record_failure(&attempt);
        sqlx::query(
            "UPDATE player_password_resets SET attempts = attempts + 1 WHERE player_id = $1",
        )
        .bind(account.id)
        .execute(&state.db)
        .await?;
        return Err(invalid());
    }

    let hash = password::hash(req.new_password).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE players SET password_hash = $2, password_changed_at = date_trunc('second', now()),
             updated_at = now()
         WHERE id = $1",
    )
    .bind(account.id)
    .bind(&hash)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM player_password_resets WHERE player_id = $1")
        .bind(account.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
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

/// 6 random digits, from the system's secure random source (via UUID v4).
fn new_code() -> String {
    let bytes = Uuid::new_v4().into_bytes();
    let n = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % 1_000_000;
    format!("{n:06}")
}

fn reset_email(to: &str, name: &str, code: &str) -> Email {
    let text = format!(
        "Hi {name},\n\nYour Havoc password reset code is: {code}\n\nEnter it in the Havoc app within {CODE_TTL_MINUTES} minutes to choose a new password. If you didn't ask for this, you can ignore this email: your password stays the same.\n\nHavoc Tactical Laser Tag"
    );
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let html = format!(
        "<div style=\"font-family:Arial,sans-serif;font-size:15px;line-height:1.5\">\
         <p>Hi {},</p><p>Your Havoc password reset code is:</p>\
         <p style=\"font-size:28px;font-weight:700;letter-spacing:6px\">{code}</p>\
         <p>Enter it in the Havoc app within {CODE_TTL_MINUTES} minutes to choose a new password. \
         If you didn't ask for this, you can ignore this email: your password stays the same.</p>\
         <p style=\"color:#666\">Havoc Tactical Laser Tag</p></div>",
        esc(name)
    );
    Email {
        to: to.to_string(),
        subject: "Your Havoc password reset code".into(),
        text,
        html,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_six_digits() {
        for _ in 0..50 {
            let c = new_code();
            assert_eq!(c.len(), 6);
            assert!(c.chars().all(|ch| ch.is_ascii_digit()));
        }
    }

    #[test]
    fn email_has_the_code_and_escapes_the_name() {
        let e = reset_email("a@b.c", "Tom <b>", "042137");
        assert!(e.text.contains("042137") && e.html.contains("042137"));
        assert!(e.html.contains("Tom &lt;b&gt;"));
    }
}
