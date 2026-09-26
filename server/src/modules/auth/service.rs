use sqlx::PgPool;

use super::{
    dto::{LoginRequest, SignupRequest},
    jwt::{self, TokenKind},
};
use crate::{
    config::Config,
    error::{AppError, AppResult},
    modules::users::{User, repository as users},
    shared::{password, validation::normalize_handle},
};

pub async fn signup(db: &PgPool, req: SignupRequest) -> AppResult<User> {
    let req = req.validated()?;
    let hash = password::hash(req.password).await?;

    let user = users::create(db, &req.username, &req.name, &hash)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                AppError::Conflict("Username is already taken".into())
            }
            _ => AppError::Database(e),
        })?;

    tracing::info!(user_id = %user.id, username = %user.username, "user created");
    Ok(user)
}

/// Verifies credentials and returns the user with a freshly signed token.
pub async fn login(db: &PgPool, config: &Config, req: LoginRequest) -> AppResult<(User, String)> {
    let invalid = || AppError::Unauthorized("Invalid username or password".into());

    let user = users::find_by_username(db, &normalize_handle(&req.username))
        .await?
        .ok_or_else(invalid)?;

    if !password::verify(req.password, user.password_hash.clone()).await? {
        return Err(invalid());
    }

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
