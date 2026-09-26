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
