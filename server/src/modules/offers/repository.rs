use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use super::{dto::ValidOffer, model::Offer};

/// Lists offers, newest first. `pattern` is an escaped ILIKE pattern (see `like_pattern`).
pub async fn list(
    db: &PgPool,
    pattern: Option<&str>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<(Vec<Offer>, i64)> {
    let items = sqlx::query_as(
        "SELECT id, title, description, is_active, starts_on, ends_on, created_at, updated_at FROM offers
         WHERE $1::text IS NULL OR title ILIKE $1 OR description ILIKE $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3"
    )
    .bind(pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar(
        "SELECT COUNT(*) FROM offers
         WHERE $1::text IS NULL OR title ILIKE $1 OR description ILIKE $1",
    )
    .bind(pattern)
    .fetch_one(db)
    .await?;

    Ok((items, total))
}

/// Offers shown in the app on `today`: active and within their date range. Newest first.
pub async fn live(db: &PgPool, today: NaiveDate) -> sqlx::Result<Vec<Offer>> {
    sqlx::query_as(
        "SELECT id, title, description, is_active, starts_on, ends_on, created_at, updated_at FROM offers
         WHERE is_active
           AND (starts_on IS NULL OR starts_on <= $1)
           AND (ends_on IS NULL OR ends_on >= $1)
         ORDER BY created_at DESC"
    )
    .bind(today)
    .fetch_all(db)
    .await
}

pub async fn find_by_id(db: &PgPool, id: Uuid) -> sqlx::Result<Option<Offer>> {
    sqlx::query_as("SELECT id, title, description, is_active, starts_on, ends_on, created_at, updated_at FROM offers WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn create(db: &PgPool, o: &ValidOffer) -> sqlx::Result<Offer> {
    sqlx::query_as(
        "INSERT INTO offers (title, description, is_active, starts_on, ends_on)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, title, description, is_active, starts_on, ends_on, created_at, updated_at",
    )
    .bind(&o.title)
    .bind(&o.description)
    .bind(o.is_active)
    .bind(o.starts_on)
    .bind(o.ends_on)
    .fetch_one(db)
    .await
}

pub async fn update(db: &PgPool, id: Uuid, o: &ValidOffer) -> sqlx::Result<Option<Offer>> {
    sqlx::query_as(
        "UPDATE offers SET
             title = $2,
             description = $3,
             is_active = $4,
             starts_on = $5,
             ends_on = $6,
             updated_at = now()
         WHERE id = $1
         RETURNING id, title, description, is_active, starts_on, ends_on, created_at, updated_at",
    )
    .bind(id)
    .bind(&o.title)
    .bind(&o.description)
    .bind(o.is_active)
    .bind(o.starts_on)
    .bind(o.ends_on)
    .fetch_optional(db)
    .await
}

/// Returns `true` if a row was deleted.
pub async fn delete(db: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM offers WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(result.rows_affected() > 0)
}
