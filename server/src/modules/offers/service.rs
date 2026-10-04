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
    shared::pagination::{PageParams, Paginated, like_pattern},
};

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
    repository::update(db, id, &offer)
        .await?
        .ok_or_else(not_found)
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
