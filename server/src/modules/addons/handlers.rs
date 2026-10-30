use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, State},
    http::{HeaderValue, Response, header},
};

use super::service::{self, Addon};
use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

/// Every add-on Bookeo offers, with its picture.
pub async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Addon>>> {
    Ok(Json(service::list(&state).await?))
}

/// `multipart/form-data` with the image in a field named `file`. Returns all add-ons.
pub async fn upload_image(
    State(state): State<AppState>,
    Path(key): Path<String>,
    mut multipart: Multipart,
) -> AppResult<Json<Vec<Addon>>> {
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
            return Ok(Json(service::set_image(&state, &key, &bytes).await?));
        }
    }
    Err(AppError::BadRequest("No image in the upload".into()))
}

pub async fn remove_image(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> AppResult<Json<Vec<Addon>>> {
    Ok(Json(service::remove_image(&state, &key).await?))
}

/// The picture (public). The URL carries a version (`?v=`), so it's cached for a year.
pub async fn serve_image(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> AppResult<Response<Body>> {
    let (content_type, data) = service::image(&state.db, &key).await?;
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
