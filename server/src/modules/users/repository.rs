use sqlx::PgPool;
use uuid::Uuid;

use super::User;

const COLUMNS: &str = "id, username, name, email, is_owner, password_hash, created_at";

pub async fn find_by_username(db: &PgPool, username: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM users WHERE username = $1"
    )))
    .bind(username)
    .fetch_optional(db)
    .await
}

/// By username or email address (staff "forgot password").
pub async fn find_by_login(db: &PgPool, login: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM users WHERE username = lower($1) OR lower(email) = lower($1) LIMIT 1"
    )))
    .bind(login.trim())
    .fetch_optional(db)
    .await
}

pub async fn find_by_id(db: &PgPool, id: Uuid) -> sqlx::Result<Option<User>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM users WHERE id = $1"
    )))
    .bind(id)
    .fetch_optional(db)
    .await
}

/// Every staff account, the owner first.
pub async fn list(db: &PgPool) -> sqlx::Result<Vec<User>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM users ORDER BY is_owner DESC, created_at"
    )))
    .fetch_all(db)
    .await
}

pub async fn create(
    db: &PgPool,
    username: &str,
    name: &str,
    email: Option<&str>,
    password_hash: &str,
) -> sqlx::Result<User> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO users (username, name, email, password_hash) VALUES ($1, $2, $3, $4)
         RETURNING {COLUMNS}"
    )))
    .bind(username)
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .fetch_one(db)
    .await
}

pub async fn update_profile(
    db: &PgPool,
    id: Uuid,
    name: &str,
    email: Option<&str>,
) -> sqlx::Result<Option<User>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE users SET name = $2, email = $3, updated_at = now() WHERE id = $1 RETURNING {COLUMNS}"
    )))
    .bind(id)
    .bind(name)
    .bind(email)
    .fetch_optional(db)
    .await
}

/// Sets a new password; every existing session of that account ends (see shared::jwt).
pub async fn set_password(db: &PgPool, id: Uuid, password_hash: &str) -> sqlx::Result<bool> {
    Ok(sqlx::query(
        "UPDATE users SET password_hash = $2, password_changed_at = date_trunc('second', now()),
             updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(password_hash)
    .execute(db)
    .await?
    .rows_affected()
        > 0)
}

/// Removes a staff account (never the owner's).
pub async fn delete(db: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    Ok(
        sqlx::query("DELETE FROM users WHERE id = $1 AND NOT is_owner")
            .bind(id)
            .execute(db)
            .await?
            .rows_affected()
            > 0,
    )
}

/// For session checks. `None` if the user no longer exists.
pub async fn password_changed_at(
    db: &PgPool,
    id: Uuid,
) -> sqlx::Result<Option<chrono::DateTime<chrono::Utc>>> {
    sqlx::query_scalar("SELECT password_changed_at FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

/// Creates the very first admin, who becomes the owner; returns `None` (and creates nothing) if
/// any admin exists. The table lock serializes concurrent attempts, so only one can ever succeed.
pub async fn create_first(
    db: &PgPool,
    username: &str,
    name: &str,
    password_hash: &str,
) -> sqlx::Result<Option<User>> {
    let mut tx = db.begin().await?;
    // Serializes concurrent bootstrap attempts.
    sqlx::query("LOCK TABLE users IN SHARE ROW EXCLUSIVE MODE")
        .execute(&mut *tx)
        .await?;
    let user = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO users (username, name, password_hash, is_owner)
         SELECT $1, $2, $3, true WHERE NOT EXISTS (SELECT 1 FROM users)
         RETURNING {COLUMNS}"
    )))
    .bind(username)
    .bind(name)
    .bind(password_hash)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(user)
}
