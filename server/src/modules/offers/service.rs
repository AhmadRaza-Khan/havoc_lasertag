use chrono::{NaiveDate, Utc};
use chrono_tz::Tz;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    dto::{ListQuery, OfferInput},
    model::Offer,
    repository,
};
use crate::{
    error::{AppError, AppResult},
    shared::{
        image::sniff,
        pagination::{PageParams, Paginated, like_pattern},
    },
};

/// Largest accepted image. The admin panel resizes before uploading (to ~1600 px JPEG, a few
/// hundred KB); nginx caps request bodies at 1 MB.
pub const MAX_IMAGE_BYTES: usize = 900 * 1024;

pub async fn list(db: &PgPool, query: ListQuery) -> AppResult<Paginated<Offer>> {
    let params = PageParams {
        page: query.page,
        per_page: query.per_page,
    };
    let pattern = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(like_pattern);

    let (items, total) =
        repository::list(db, pattern.as_deref(), params.per_page(), params.offset()).await?;
    Ok(Paginated::new(items, total, &params))
}

/// Offers to show in the app right now; "today" is the venue's local date.
pub async fn live(db: &PgPool, timezone: Tz) -> AppResult<Vec<Offer>> {
    let today: NaiveDate = Utc::now().with_timezone(&timezone).date_naive();
    Ok(repository::live(db, today).await?)
}

pub async fn get(db: &PgPool, id: Uuid) -> AppResult<Offer> {
    repository::find_by_id(db, id).await?.ok_or_else(not_found)
}

pub async fn create(db: &PgPool, input: OfferInput) -> AppResult<Offer> {
    let offer = input.validate()?;
    let created = repository::create(db, &offer).await?;
    tracing::info!(offer_id = %created.id, "offer created");
    Ok(created)
}

pub async fn update(db: &PgPool, id: Uuid, input: OfferInput) -> AppResult<Offer> {
    let offer = input.validate()?;
    // Clearing the text is fine only if the offer still has an image.
    if !offer.has_text() && get(db, id).await?.image_url.is_none() {
        return Err(nothing_to_show());
    }
    repository::update(db, id, &offer)
        .await?
        .ok_or_else(not_found)
}

pub async fn set_image(db: &PgPool, id: Uuid, bytes: &[u8]) -> AppResult<Offer> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(AppError::BadRequest(
            "The image is too large (max 900 KB)".into(),
        ));
    }
    let content_type =
        sniff(bytes).ok_or_else(|| AppError::BadRequest("Use a JPEG, PNG or WebP image".into()))?;
    let offer = repository::set_image(db, id, content_type, bytes)
        .await?
        .ok_or_else(not_found)?;
    tracing::info!(offer_id = %id, bytes = bytes.len(), "offer image updated");
    Ok(offer)
}

pub async fn remove_image(db: &PgPool, id: Uuid) -> AppResult<Offer> {
    if !get(db, id).await?.has_text() {
        return Err(nothing_to_show());
    }
    repository::remove_image(db, id)
        .await?
        .ok_or_else(not_found)
}

pub async fn image(db: &PgPool, id: Uuid) -> AppResult<(String, Vec<u8>)> {
    repository::image(db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("No image".into()))
}

fn nothing_to_show() -> AppError {
    AppError::BadRequest("An offer needs a title, a description or an image".into())
}

pub async fn delete(db: &PgPool, id: Uuid) -> AppResult<()> {
    if !repository::delete(db, id).await? {
        return Err(not_found());
    }
    tracing::info!(offer_id = %id, "offer deleted");
    Ok(())
}

fn not_found() -> AppError {
    AppError::NotFound("Offer not found".into())
}
