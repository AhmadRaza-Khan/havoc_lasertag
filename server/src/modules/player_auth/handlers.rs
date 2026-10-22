use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};

use super::AuthPlayer;
use crate::{
    error::{AppError, AppResult},
    modules::{
        chats,
        players::{self as players_module, Player, SignupInput, repository as players},
        stats,
    },
    shared::{
        jwt::{self, TokenKind},
        login_throttle::Attempt,
        password,
        validation::normalize_handle,
    },
    state::AppState,
};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub playername: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub player: Player,
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let invalid = || AppError::Unauthorized("Invalid playername or password".into());
    let playername = normalize_handle(&req.playername);
    let attempt = Attempt::new("player", &playername, &headers);
    state.login_throttle.check(&attempt)?;

    let credentials = players::find_credentials(&state.db, &playername).await?;
    // Same cost whether or not the account exists (no playername enumeration by timing).
    let hash = credentials.as_ref().map(|(_, h)| h.clone());
    if !password::verify_or_dummy(req.password, hash).await? {
        state.login_throttle.record_failure(&attempt);
        return Err(invalid());
    }
    let (id, _) = credentials.ok_or_else(invalid)?;
    state.login_throttle.record_success(&attempt);

    let token = jwt::create_token(
        id,
        TokenKind::Player,
        &state.config.jwt_secret,
        state.config.jwt_ttl_hours,
    )?;
    let player = current_player(&state, id).await?;

    Ok(Json(LoginResponse { token, player }))
}

/// Creates a player account from the app and signs it in. Admins can still create players
/// in the panel. At most a few sign-ups per network per window (see `login_throttle`).
pub async fn signup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<SignupInput>,
) -> AppResult<(StatusCode, Json<LoginResponse>)> {
    let attempt = Attempt::per_client("player-signup", &headers);
    state.login_throttle.check(&attempt).map_err(|_| {
        AppError::TooManyRequests(
            "Too many new accounts from this network. Try again in a few minutes.".into(),
        )
    })?;

    let player = players_module::signup(&state.db, input).await?;
    // Only successful sign-ups count towards the limit (typos in the form don't).
    state.login_throttle.record_failure(&attempt);
    tracing::info!(player_id = %player.id, playername = %player.playername, "player signed up in the app");
    // Stats from earlier uploads that listed this name become theirs.
    stats::link_unclaimed_records(&state, player.id, &player.playername).await;

    let token = jwt::create_token(
        player.id,
        TokenKind::Player,
        &state.config.jwt_secret,
        state.config.jwt_ttl_hours,
    )?;
    Ok((StatusCode::CREATED, Json(LoginResponse { token, player })))
}

#[derive(Deserialize)]
pub struct AvailabilityQuery {
    pub playername: String,
}

#[derive(Serialize)]
pub struct Availability {
    /// The name as it would be stored (lowercase, trimmed).
    pub playername: String,
    pub available: bool,
    /// Why it can't be used (invalid or taken), when `available` is false.
    pub reason: Option<String>,
}

/// For the sign-up form: is this playername valid and free?
pub async fn playername_available(
    State(state): State<AppState>,
    Query(q): Query<AvailabilityQuery>,
) -> AppResult<Json<Availability>> {
    let playername = normalize_handle(&q.playername);
    Ok(Json(
        match players_module::check_playername(&state.db, &q.playername).await {
            Ok(name) => Availability {
                playername: name,
                available: true,
                reason: None,
            },
            Err(AppError::BadRequest(reason) | AppError::Conflict(reason)) => Availability {
                playername,
                available: false,
                reason: Some(reason),
            },
            Err(e) => return Err(e),
        },
    ))
}

pub async fn me(State(state): State<AppState>, auth: AuthPlayer) -> AppResult<Json<Player>> {
    Ok(Json(current_player(&state, auth.id).await?))
}

async fn current_player(state: &AppState, id: uuid::Uuid) -> AppResult<Player> {
    players::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Player no longer exists".into()))
}

#[derive(Deserialize)]
pub struct DeleteMeRequest {
    pub password: String,
}

/// `DELETE /api/player/auth/me`: the signed-in player deletes their account (App Store / Google
/// Play requirement). The password confirms it's really them.
pub async fn delete_me(
    State(state): State<AppState>,
    auth: AuthPlayer,
    headers: HeaderMap,
    Json(req): Json<DeleteMeRequest>,
) -> AppResult<StatusCode> {
    let attempt = Attempt::new("delete", &auth.id.to_string(), &headers);
    state.login_throttle.check(&attempt)?;
    let hash = players::password_hash(&state.db, auth.id).await?;
    if !password::verify_or_dummy(req.password, hash).await? {
        state.login_throttle.record_failure(&attempt);
        // 403, not 401: the session is fine, only the confirmation failed (a 401 signs the app out).
        return Err(AppError::Forbidden("Wrong password".into()));
    }
    state.login_throttle.record_success(&attempt);
    delete_account(&state, auth.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/player/auth/delete-account`: the same, from the website's deletion page (Google
/// Play requires a way to delete an account without the app).
pub async fn delete_account_by_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> AppResult<StatusCode> {
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
    delete_account(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Deletes the player and everything that's theirs (bookings here, chats, picture, push
/// tokens); game stats stay, unlinked. Their open chat sockets close at once.
async fn delete_account(state: &AppState, id: uuid::Uuid) -> AppResult<()> {
    players_module::delete_player(&state.db, id).await?;
    chats::disconnect_player(state, id);
    tracing::info!(player_id = %id, "player deleted their account");
    Ok(())
}
