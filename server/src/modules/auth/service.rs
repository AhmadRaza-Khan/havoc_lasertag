use sqlx::PgPool;

use super::dto::{LoginRequest, SignupRequest};
use crate::{
    config::Config,
    error::{AppError, AppResult},
    modules::users::{User, repository as users},
    shared::{
        jwt::{self, TokenKind},
        password,
        validation::normalize_handle,
    },
};

/// Creates an admin. Only a signed-in admin may do this — except for the very first admin,
/// which can be created without authentication while no admin exists.
pub async fn signup(db: &PgPool, req: SignupRequest, by_admin: bool) -> AppResult<User> {
    let req = req.validated()?;
    let hash = password::hash(req.password).await?;

    let created = if by_admin {
        users::create(db, &req.username, &req.name, &hash)
            .await
            .map(Some)
    } else {
        users::create_first(db, &req.username, &req.name, &hash).await
    };
    let user = created
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                AppError::Conflict("Username is already taken".into())
            }
            _ => AppError::Database(e),
        })?
        .ok_or_else(|| {
            AppError::Unauthorized("Only a signed-in admin can create admin accounts".into())
        })?;

    tracing::info!(user_id = %user.id, username = %user.username, "user created");
    Ok(user)
}

/// Verifies credentials and returns the user with a freshly signed token.
pub async fn login(db: &PgPool, config: &Config, req: LoginRequest) -> AppResult<(User, String)> {
    let invalid = || AppError::Unauthorized("Invalid username or password".into());

    let user = users::find_by_username(db, &normalize_handle(&req.username)).await?;
    // Same cost whether or not the account exists (no username enumeration by timing).
    let hash = user.as_ref().map(|u| u.password_hash.clone());
    if !password::verify_or_dummy(req.password, hash).await? {
        return Err(invalid());
    }
    let user = user.ok_or_else(invalid)?;

    let token = jwt::create_token(
        user.id,
        TokenKind::Admin,
        &config.jwt_secret,
        config.jwt_ttl_hours,
    )?;
    Ok((user, token))
}

pub async fn current_user(db: &PgPool, user_id: uuid::Uuid) -> AppResult<User> {
    users::find_by_id(db, user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User no longer exists".into()))
}
