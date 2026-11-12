//! Staff accounts. Everyone manages their own name, email and password; the owner also adds,
//! edits and removes staff accounts and can set any staff password by hand.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use super::{PublicUser, User, repository as users};
use crate::{
    error::{AppError, AppResult},
    modules::auth::{self, AuthUser},
    shared::{
        login_throttle::Attempt,
        password,
        validation::{
            normalize_handle, required_text, validate_email, validate_handle, validate_password,
        },
    },
    state::AppState,
};

/// `GET /api/users`: the owner sees every staff account; others see their own.
pub async fn list(State(state): State<AppState>, me: AuthUser) -> AppResult<Json<Vec<PublicUser>>> {
    let me = load(&state, me.id).await?;
    let list = if me.is_owner {
        users::list(&state.db).await?
    } else {
        vec![me]
    };
    Ok(Json(list.into_iter().map(Into::into).collect()))
}

#[derive(Deserialize)]
pub struct CreateRequest {
    pub username: String,
    pub name: String,
    pub email: Option<String>,
    pub password: String,
}

/// `POST /api/users` (owner): a new staff account.
pub async fn create(
    State(state): State<AppState>,
    me: AuthUser,
    Json(req): Json<CreateRequest>,
) -> AppResult<(StatusCode, Json<PublicUser>)> {
    owner(&state, &me).await?;
    let username = normalize_handle(&req.username);
    validate_handle("Username", &username)?;
    let name = required_text("Name", &req.name)?;
    let email = email_input(req.email)?;
    validate_password(&req.password)?;
    let hash = password::hash(req.password).await?;
    let user = users::create(&state.db, &username, &name, email.as_deref(), &hash)
        .await
        .map_err(unique_violation)?;
    tracing::info!(user_id = %user.id, by = %me.id, "staff account created");
    Ok((StatusCode::CREATED, Json(user.into())))
}

#[derive(Deserialize)]
pub struct ProfileRequest {
    pub name: String,
    /// Empty removes it.
    pub email: Option<String>,
}

/// `PATCH /api/users/me`: your own name and email.
pub async fn update_me(
    State(state): State<AppState>,
    me: AuthUser,
    Json(req): Json<ProfileRequest>,
) -> AppResult<Json<PublicUser>> {
    save_profile(&state, me.id, req).await
}

/// `PATCH /api/users/{id}` (owner): any staff account's name and email.
pub async fn update(
    State(state): State<AppState>,
    me: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ProfileRequest>,
) -> AppResult<Json<PublicUser>> {
    owner(&state, &me).await?;
    save_profile(&state, id, req).await
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// `POST /api/users/me/password`: needs your current password. Your other sessions end; this
/// browser gets a fresh session cookie.
pub async fn change_my_password(
    State(state): State<AppState>,
    me: AuthUser,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<(CookieJar, StatusCode)> {
    let attempt = Attempt::new("staff-change-password", &me.id.to_string(), &headers);
    state.login_throttle.check(&attempt)?;
    let user = load(&state, me.id).await?;
    if !password::verify(req.current_password, user.password_hash).await? {
        state.login_throttle.record_failure(&attempt);
        // 403, not 401: the session is fine, only the confirmation failed.
        return Err(AppError::Forbidden("Your current password is wrong".into()));
    }
    validate_password(&req.new_password)?;
    state.login_throttle.record_success(&attempt);
    let hash = password::hash(req.new_password).await?;
    users::set_password(&state.db, me.id, &hash).await?;
    tracing::info!(user_id = %me.id, "staff member changed their password");
    Ok((
        jar.add(auth::new_session_cookie(&state, me.id)?),
        StatusCode::NO_CONTENT,
    ))
}

#[derive(Deserialize)]
pub struct SetPasswordRequest {
    pub password: String,
}

/// `PUT /api/users/{id}/password` (owner): sets a staff member's password by hand. Their
/// sessions end; tell them the new password.
pub async fn set_password(
    State(state): State<AppState>,
    me: AuthUser,
    jar: CookieJar,
    Path(id): Path<Uuid>,
    Json(req): Json<SetPasswordRequest>,
) -> AppResult<(CookieJar, StatusCode)> {
    owner(&state, &me).await?;
    validate_password(&req.password)?;
    let hash = password::hash(req.password).await?;
    if !users::set_password(&state.db, id, &hash).await? {
        return Err(not_found());
    }
    tracing::info!(user_id = %id, by = %me.id, "staff password set by the owner");
    // Setting your own this way keeps you signed in.
    let jar = if id == me.id {
        jar.add(auth::new_session_cookie(&state, me.id)?)
    } else {
        jar
    };
    Ok((jar, StatusCode::NO_CONTENT))
}

/// `DELETE /api/users/{id}` (owner): removes a staff account (not the owner's).
pub async fn delete(
    State(state): State<AppState>,
    me: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    owner(&state, &me).await?;
    if id == me.id || !users::delete(&state.db, id).await? {
        return Err(AppError::Conflict(
            "The owner's account can't be removed".into(),
        ));
    }
    tracing::info!(user_id = %id, by = %me.id, "staff account removed");
    Ok(StatusCode::NO_CONTENT)
}

async fn save_profile(
    state: &AppState,
    id: Uuid,
    req: ProfileRequest,
) -> AppResult<Json<PublicUser>> {
    let name = required_text("Name", &req.name)?;
    let email = email_input(req.email)?;
    let user = users::update_profile(&state.db, id, &name, email.as_deref())
        .await
        .map_err(unique_violation)?
        .ok_or_else(not_found)?;
    Ok(Json(user.into()))
}

/// Trimmed and checked; empty means none.
fn email_input(email: Option<String>) -> AppResult<Option<String>> {
    match email
        .map(|e| e.trim().to_string())
        .filter(|e| !e.is_empty())
    {
        Some(e) => {
            validate_email(&e)?;
            Ok(Some(e))
        }
        None => Ok(None),
    }
}

async fn load(state: &AppState, id: Uuid) -> AppResult<User> {
    users::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User no longer exists".into()))
}

async fn owner(state: &AppState, me: &AuthUser) -> AppResult<User> {
    let user = load(state, me.id).await?;
    if !user.is_owner {
        return Err(AppError::Forbidden(
            "Only the owner can manage staff accounts".into(),
        ));
    }
    Ok(user)
}

fn unique_violation(e: sqlx::Error) -> AppError {
    match &e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            let what = if db.constraint().is_some_and(|c| c.contains("email")) {
                "That email is already used by another staff account"
            } else {
                "That username is already taken"
            };
            AppError::Conflict(what.into())
        }
        _ => AppError::Database(e),
    }
}

fn not_found() -> AppError {
    AppError::NotFound("Staff account not found".into())
}
