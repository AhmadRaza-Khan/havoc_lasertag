use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use super::{dto::ValidOffer, model::Offer};

const COLUMNS: &str = "id, title, description, is_active, starts_on, ends_on,
    CASE WHEN image_updated_at IS NULL THEN NULL
         ELSE '/api/offer-images/' || id || '?v=' || floor(extract(epoch FROM image_updated_at) * 1000)::bigint
    END AS image_url,
    created_at, updated_at";

/// Lists offers, newest first. `pattern` is an escaped ILIKE pattern (see `like_pattern`).
pub async fn list(
    db: &PgPool,
    pattern: Option<&str>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<(Vec<Offer>, i64)> {
    let items = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM offers
         WHERE $1::text IS NULL OR title ILIKE $1 OR description ILIKE $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3"
    )))
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

/// Offers shown in the app on `today`: active, within their date range, and with something to
/// show (text or an image). Newest first.
pub async fn live(db: &PgPool, today: NaiveDate) -> sqlx::Result<Vec<Offer>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM offers
         WHERE is_active
           AND (starts_on IS NULL OR starts_on <= $1)
           AND (ends_on IS NULL OR ends_on >= $1)
           AND (title <> '' OR description <> '' OR image_updated_at IS NOT NULL)
         ORDER BY created_at DESC"
    )))
    .bind(today)
    .fetch_all(db)
    .await
}

pub async fn find_by_id(db: &PgPool, id: Uuid) -> sqlx::Result<Option<Offer>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM offers WHERE id = $1"
    )))
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn create(db: &PgPool, o: &ValidOffer) -> sqlx::Result<Offer> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO offers (title, description, is_active, starts_on, ends_on)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING {COLUMNS}"
    )))
    .bind(&o.title)
    .bind(&o.description)
    .bind(o.is_active)
    .bind(o.starts_on)
    .bind(o.ends_on)
    .fetch_one(db)
    .await
}

pub async fn update(db: &PgPool, id: Uuid, o: &ValidOffer) -> sqlx::Result<Option<Offer>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE offers SET
             title = $2,
             description = $3,
             is_active = $4,
             starts_on = $5,
             ends_on = $6,
             updated_at = now()
         WHERE id = $1
         RETURNING {COLUMNS}"
    )))
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

/// Stores (or replaces) the offer's image. `None` if there's no such offer.
pub async fn set_image(
    db: &PgPool,
    id: Uuid,
    content_type: &str,
    data: &[u8],
) -> sqlx::Result<Option<Offer>> {
    let mut tx = db.begin().await?;
    let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM offers WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    if exists.is_none() {
        return Ok(None);
    }
    sqlx::query(
        "INSERT INTO offer_images (offer_id, content_type, data) VALUES ($1, $2, $3)
         ON CONFLICT (offer_id) DO UPDATE SET content_type = $2, data = $3, updated_at = now()",
    )
    .bind(id)
    .bind(content_type)
    .bind(data)
    .execute(&mut *tx)
    .await?;
    let offer = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE offers SET image_updated_at = now(), updated_at = now() WHERE id = $1
         RETURNING {COLUMNS}"
    )))
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Some(offer))
}

/// Removes the offer's image. `None` if there's no such offer.
pub async fn remove_image(db: &PgPool, id: Uuid) -> sqlx::Result<Option<Offer>> {
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM offer_images WHERE offer_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let offer = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE offers SET image_updated_at = NULL, updated_at = now() WHERE id = $1
         RETURNING {COLUMNS}"
    )))
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(offer)
}

/// The image's type and bytes.
pub async fn image(db: &PgPool, id: Uuid) -> sqlx::Result<Option<(String, Vec<u8>)>> {
    sqlx::query_as("SELECT content_type, data FROM offer_images WHERE offer_id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}
