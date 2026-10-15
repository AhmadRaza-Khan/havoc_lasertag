use sqlx::PgPool;
use uuid::Uuid;

use super::{dto::ValidPlayer, model::Player};

/// Lists players, newest first. `pattern` is an escaped ILIKE pattern (see `like_pattern`).
pub async fn list(
    db: &PgPool,
    pattern: Option<&str>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<(Vec<Player>, i64)> {
    let items = sqlx::query_as(
        "SELECT id, full_name, playername, phone, email, created_at, updated_at,
                CASE WHEN avatar_updated_at IS NULL THEN NULL ELSE '/api/avatars/' || id || '?v=' || floor(extract(epoch FROM avatar_updated_at) * 1000)::bigint END AS avatar_url
         FROM players
         WHERE $1::text IS NULL OR playername ILIKE $1 OR full_name ILIKE $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar(
        "SELECT COUNT(*) FROM players
         WHERE $1::text IS NULL OR playername ILIKE $1 OR full_name ILIKE $1",
    )
    .bind(pattern)
    .fetch_one(db)
    .await?;

    Ok((items, total))
}

pub async fn find_by_id(db: &PgPool, id: Uuid) -> sqlx::Result<Option<Player>> {
    sqlx::query_as(
        "SELECT id, full_name, playername, phone, email, created_at, updated_at,
                CASE WHEN avatar_updated_at IS NULL THEN NULL ELSE '/api/avatars/' || id || '?v=' || floor(extract(epoch FROM avatar_updated_at) * 1000)::bigint END AS avatar_url
         FROM players WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn create(db: &PgPool, p: &ValidPlayer, password_hash: &str) -> sqlx::Result<Player> {
    sqlx::query_as(
        "INSERT INTO players (full_name, playername, phone, email, password_hash)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, full_name, playername, phone, email, created_at, updated_at,
                CASE WHEN avatar_updated_at IS NULL THEN NULL ELSE '/api/avatars/' || id || '?v=' || floor(extract(epoch FROM avatar_updated_at) * 1000)::bigint END AS avatar_url",
    )
    .bind(&p.full_name)
    .bind(&p.playername)
    .bind(&p.phone)
    .bind(&p.email)
    .bind(password_hash)
    .fetch_one(db)
    .await
}

/// Updates a player. A `None` password hash keeps the existing password.
pub async fn update(
    db: &PgPool,
    id: Uuid,
    p: &ValidPlayer,
    password_hash: Option<&str>,
) -> sqlx::Result<Option<Player>> {
    sqlx::query_as(
        "UPDATE players SET
             full_name = $2,
             playername = $3,
             phone = $4,
             email = $5,
             password_hash = COALESCE($6, password_hash),
             -- A new password signs out every existing session (see shared::jwt).
             password_changed_at = CASE WHEN $6 IS NULL THEN password_changed_at
                                        ELSE date_trunc('second', now()) END,
             updated_at = now()
         WHERE id = $1
         RETURNING id, full_name, playername, phone, email, created_at, updated_at,
                CASE WHEN avatar_updated_at IS NULL THEN NULL ELSE '/api/avatars/' || id || '?v=' || floor(extract(epoch FROM avatar_updated_at) * 1000)::bigint END AS avatar_url",
    )
    .bind(id)
    .bind(&p.full_name)
    .bind(&p.playername)
    .bind(&p.phone)
    .bind(&p.email)
    .bind(password_hash)
    .fetch_optional(db)
    .await
}

/// Returns `true` if a row was deleted.
pub async fn delete(db: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM players WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Id and password hash for login. Playername must already be normalized.
pub async fn find_credentials(
    db: &PgPool,
    playername: &str,
) -> sqlx::Result<Option<(Uuid, String)>> {
    sqlx::query_as("SELECT id, password_hash FROM players WHERE playername = $1")
        .bind(playername)
        .fetch_optional(db)
        .await
}

pub async fn bookeo_customer_id(db: &PgPool, id: Uuid) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT bookeo_customer_id FROM players WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
        .map(Option::flatten)
}

/// Stores the Bookeo customer id unless another request already set one; returns the id in effect.
pub async fn set_bookeo_customer_id(
    db: &PgPool,
    id: Uuid,
    customer_id: &str,
) -> sqlx::Result<String> {
    sqlx::query_scalar(
        "UPDATE players SET bookeo_customer_id = COALESCE(bookeo_customer_id, $2)
         WHERE id = $1
         RETURNING bookeo_customer_id",
    )
    .bind(id)
    .bind(customer_id)
    .fetch_one(db)
    .await
}

/// For session checks. `None` if the player no longer exists.
pub async fn password_changed_at(
    db: &PgPool,
    id: Uuid,
) -> sqlx::Result<Option<chrono::DateTime<chrono::Utc>>> {
    sqlx::query_scalar("SELECT password_changed_at FROM players WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}
