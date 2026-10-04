use sqlx::PgPool;
use uuid::Uuid;

use super::User;

pub async fn find_by_username(db: &PgPool, username: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as(
        "SELECT id, username, name, password_hash, created_at FROM users WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(db)
    .await
}

pub async fn find_by_id(db: &PgPool, id: Uuid) -> sqlx::Result<Option<User>> {
    sqlx::query_as("SELECT id, username, name, password_hash, created_at FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn create(
    db: &PgPool,
    username: &str,
    name: &str,
    password_hash: &str,
) -> sqlx::Result<User> {
    sqlx::query_as(
        "INSERT INTO users (username, name, password_hash) VALUES ($1, $2, $3) \
         RETURNING id, username, name, password_hash, created_at",
    )
    .bind(username)
    .bind(name)
    .bind(password_hash)
    .fetch_one(db)
    .await
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

/// Creates the very first admin; returns `None` (and creates nothing) if any admin exists.
/// The table lock serializes concurrent attempts, so only one can ever succeed.
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
    let user = sqlx::query_as(
        "INSERT INTO users (username, name, password_hash)
         SELECT $1, $2, $3 WHERE NOT EXISTS (SELECT 1 FROM users)
         RETURNING id, username, name, password_hash, created_at",
    )
    .bind(username)
    .bind(name)
    .bind(password_hash)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(user)
}
