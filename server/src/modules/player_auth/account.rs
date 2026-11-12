//! A signed-in player's own account: verifying their email, changing their password and editing
//! their profile. Plus the website's account deletion, confirmed with a code emailed to the
//! player's verified address.

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};

use super::{
    AuthPlayer,
    handlers::{LoginResponse, delete_account},
};
use crate::{
    error::{AppError, AppResult},
    modules::{
        chats,
        players::{self as players_module, Player, PlayerInput, repository as players},
    },
    shared::{
        email_code::{self, Purpose},
        jwt::{self, TokenKind},
        login_throttle::Attempt,
        password,
        validation::{normalize_handle, validate_password},
    },
    state::AppState,
};

/// Emails a verification code to the player's address. Used at sign-up and when they change
/// their email; `false` if one was sent under a minute ago.
pub async fn send_verification(state: &AppState, player: &Player) -> AppResult<bool> {
    let Some(email) = player.email.as_deref().filter(|e| !e.trim().is_empty()) else {
        return Err(AppError::Conflict(
            "Add an email address to your account first".into(),
        ));
    };
    let Some(code) = email_code::issue(&state.db, player.id, Purpose::PlayerVerify).await? else {
        return Ok(false);
    };
    state.mailer.send_later(email_code::email(
        email,
        &player.full_name,
        "Confirm your Havoc email",
        "Your Havoc email confirmation code is",
        &code,
    ));
    tracing::info!(player_id = %player.id, "email verification code sent");
    Ok(true)
}

#[derive(Serialize)]
pub struct CodeSent {
    /// False when no new code was sent: one went out under a minute ago, or (`if_none`) one is
    /// still pending. The earlier code still works.
    pub sent: bool,
}

#[derive(Deserialize)]
pub struct SendCodeQuery {
    /// Only send if there's no working code yet: the app asks this when the confirmation screen
    /// opens, so accounts made before verification existed get a code, and a code just sent at
    /// sign-up isn't replaced.
    #[serde(default)]
    pub if_none: bool,
}

/// `POST /api/player/auth/email/send-code`: (again) emails the code that confirms the address.
/// `?if_none=true`: only if no working code is pending.
pub async fn send_email_code(
    State(state): State<AppState>,
    auth: AuthPlayer,
    Query(q): Query<SendCodeQuery>,
) -> AppResult<Json<CodeSent>> {
    let player = current(&state, &auth).await?;
    if player.email_verified {
        return Err(AppError::Conflict("Your email is already confirmed".into()));
    }
    if q.if_none && email_code::has_pending(&state.db, player.id, Purpose::PlayerVerify).await? {
        return Ok(Json(CodeSent { sent: false }));
    }
    Ok(Json(CodeSent {
        sent: send_verification(&state, &player).await?,
    }))
}

#[derive(Deserialize)]
pub struct CodeRequest {
    pub code: String,
}

/// `POST /api/player/auth/email/verify`: confirms the address with the emailed code.
pub async fn verify_email(
    State(state): State<AppState>,
    auth: AuthPlayer,
    headers: HeaderMap,
    Json(req): Json<CodeRequest>,
) -> AppResult<Json<Player>> {
    let attempt = Attempt::new("verify-email", &auth.id.to_string(), &headers);
    state.login_throttle.check(&attempt)?;
    if !email_code::check(&state.db, auth.id, Purpose::PlayerVerify, &req.code).await? {
        state.login_throttle.record_failure(&attempt);
        return Err(AppError::BadRequest(
            "That code is wrong or has expired. Ask for a new one.".into(),
        ));
    }
    state.login_throttle.record_success(&attempt);
    players::set_email_verified(&state.db, auth.id).await?;
    tracing::info!(player_id = %auth.id, "email verified");
    Ok(Json(current(&state, &auth).await?))
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// `POST /api/player/auth/change-password`: needs the current password. Other phones are signed
/// out; this one gets a fresh token.
pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthPlayer,
    headers: HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<Json<LoginResponse>> {
    let attempt = Attempt::new("change-password", &auth.id.to_string(), &headers);
    state.login_throttle.check(&attempt)?;
    let hash = players::password_hash(&state.db, auth.id).await?;
    if !password::verify_or_dummy(req.current_password, hash).await? {
        state.login_throttle.record_failure(&attempt);
        // 403, not 401: the session is fine, only the confirmation failed (a 401 signs the app out).
        return Err(AppError::Forbidden("Your current password is wrong".into()));
    }
    validate_password(&req.new_password)?;
    state.login_throttle.record_success(&attempt);

    let new_hash = password::hash(req.new_password).await?;
    players::set_password(&state.db, auth.id, &new_hash).await?;
    // Sessions on other phones end; live chat reconnects with the new token.
    chats::disconnect_player(&state, auth.id);
    tracing::info!(player_id = %auth.id, "player changed their password");

    let token = jwt::create_token(
        auth.id,
        TokenKind::Player,
        &state.config.jwt_secret,
        state.config.jwt_ttl_hours,
    )?;
    Ok(Json(LoginResponse {
        token,
        player: current(&state, &auth).await?,
    }))
}

#[derive(Deserialize)]
pub struct ProfileRequest {
    pub full_name: String,
    pub email: String,
    pub phone: String,
}

/// `PATCH /api/player/auth/me`: name, email and phone (the playername stays: it's the
/// player's identity on the leaderboard). A new email has to be confirmed again.
pub async fn update_me(
    State(state): State<AppState>,
    auth: AuthPlayer,
    Json(req): Json<ProfileRequest>,
) -> AppResult<Json<Player>> {
    let before = current(&state, &auth).await?;
    let required = |label: &str, v: &str| {
        if v.trim().is_empty() {
            Err(AppError::BadRequest(format!("{label} is required")))
        } else {
            Ok(())
        }
    };
    required("Email", &req.email)?;
    required("Phone number", &req.phone)?;
    let updated = players_module::update_player(
        &state.db,
        auth.id,
        PlayerInput {
            full_name: req.full_name,
            playername: before.playername.clone(),
            phone: Some(req.phone),
            email: Some(req.email),
            password: None,
        },
    )
    .await?;
    if !updated.email_verified && updated.email != before.email {
        send_verification(&state, &updated).await?;
    }
    tracing::info!(player_id = %auth.id, "player updated their profile");
    Ok(Json(updated))
}

#[derive(Deserialize)]
pub struct DeleteRequest {
    pub playername: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct DeleteCodeSent {
    /// Where the code went, partly hidden, e.g. "t••@example.com".
    pub email_hint: String,
}

/// `POST /api/player/auth/delete-account`: the website's account deletion, step 1. Checks the
/// playername and password, then emails a code to the player's verified address.
pub async fn request_deletion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<DeleteRequest>,
) -> AppResult<Json<DeleteCodeSent>> {
    let invalid = || AppError::Unauthorized("Invalid playername or password".into());
    let playername = normalize_handle(&req.playername);
    let attempt = Attempt::new("delete", &playername, &headers);
    state.login_throttle.check(&attempt)?;
    let credentials = players::find_credentials(&state.db, &playername).await?;
    let hash = credentials.as_ref().map(|(_, h)| h.clone());
    if !password::verify_or_dummy(req.password, hash).await? {
        state.login_throttle.record_failure(&attempt);
        return Err(invalid());
    }
    let (id, _) = credentials.ok_or_else(invalid)?;
    state.login_throttle.record_success(&attempt);

    let player = players::find_by_id(&state.db, id)
        .await?
        .ok_or_else(invalid)?;
    let email = match (&player.email, player.email_verified) {
        (Some(e), true) => e.clone(),
        _ => {
            return Err(AppError::Conflict(
                "Your account's email address isn't confirmed, so we can't send you a code. \
                 Delete your account in the Havoc app (Profile → Delete account), or contact us."
                    .into(),
            ));
        }
    };
    if let Some(code) = email_code::issue(&state.db, id, Purpose::PlayerDelete).await? {
        state.mailer.send_later(email_code::email(
            &email,
            &player.full_name,
            "Confirm deleting your Havoc account",
            "To delete your Havoc account permanently, enter this code on the deletion page",
            &code,
        ));
        tracing::info!(player_id = %id, "account deletion code sent");
    }
    Ok(Json(DeleteCodeSent {
        email_hint: email_code::hint(&email),
    }))
}

#[derive(Deserialize)]
pub struct DeleteConfirmRequest {
    pub playername: String,
    pub code: String,
}

/// `POST /api/player/auth/delete-account/confirm`: step 2, with the emailed code. Deletes the
/// account permanently.
pub async fn confirm_deletion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<DeleteConfirmRequest>,
) -> AppResult<StatusCode> {
    let invalid = || AppError::BadRequest("That code is wrong or has expired. Start again.".into());
    let playername = normalize_handle(&req.playername);
    let attempt = Attempt::new("delete-confirm", &playername, &headers);
    state.login_throttle.check(&attempt)?;
    let Some((id, _)) = players::find_credentials(&state.db, &playername).await? else {
        state.login_throttle.record_failure(&attempt);
        password::verify_or_dummy(req.code, None).await?;
        return Err(invalid());
    };
    if !email_code::check(&state.db, id, Purpose::PlayerDelete, &req.code).await? {
        state.login_throttle.record_failure(&attempt);
        return Err(invalid());
    }
    state.login_throttle.record_success(&attempt);
    delete_account(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn current(state: &AppState, auth: &AuthPlayer) -> AppResult<Player> {
    players::find_by_id(&state.db, auth.id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Player no longer exists".into()))
}
