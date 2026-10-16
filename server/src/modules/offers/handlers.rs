use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderValue, Response, StatusCode, header},
};
use uuid::Uuid;

use super::{
    dto::{ListQuery, OfferInput},
    model::Offer,
    service,
};
use crate::{
    error::{AppError, AppResult},
    shared::pagination::Paginated,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Paginated<Offer>>> {
    Ok(Json(service::list(&state.db, query).await?))
}

/// Mobile app: the offers live today.
pub async fn live(State(state): State<AppState>) -> AppResult<Json<Vec<Offer>>> {
    Ok(Json(
        service::live(&state.db, state.config.bookeo.timezone).await?,
    ))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> AppResult<Json<Offer>> {
    Ok(Json(service::get(&state.db, id).await?))
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<OfferInput>,
) -> AppResult<(StatusCode, Json<Offer>)> {
    Ok((
        StatusCode::CREATED,
        Json(service::create(&state.db, input).await?),
    ))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<OfferInput>,
) -> AppResult<Json<Offer>> {
    Ok(Json(service::update(&state.db, id, input).await?))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<Uuid>) -> AppResult<StatusCode> {
    service::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `multipart/form-data` with the image in a field named `file`. Returns the updated offer.
pub async fn upload_image(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> AppResult<Json<Offer>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid upload: {e}")))?
    {
        if field.name() == Some("file") {
            let bytes = field
                .bytes()
                .await
                .map_err(|_| AppError::BadRequest("The image is too large (max 900 KB)".into()))?;
            return Ok(Json(service::set_image(&state.db, id, &bytes).await?));
        }
    }
    Err(AppError::BadRequest("No image in the upload".into()))
}

pub async fn remove_image(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Offer>> {
    Ok(Json(service::remove_image(&state.db, id).await?))
}

/// The image (public). The URL carries a version (`?v=`), so it's cached for a year.
pub async fn serve_image(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Response<Body>> {
    let (content_type, data) = service::image(&state.db, id).await?;
    let mut res = Response::new(Body::from(data));
    let headers = res.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&content_type).unwrap_or(HeaderValue::from_static("image/jpeg")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    Ok(res)
}
