//! Admin panel: upload game reports, follow imports, see the general report.

use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use super::{
    dto::{ImportResponse, Standing},
    service,
};
use crate::{
    error::{AppError, AppResult},
    modules::auth::AuthUser,
    state::AppState,
};

/// `multipart/form-data` with the PDF in a field named `file`. Returns 202 with the import in
/// `processing` state; poll `GET /api/stats/imports/{id}` until it completes or fails.
pub async fn upload(
    State(state): State<AppState>,
    admin: AuthUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<ImportResponse>)> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid upload: {e}")))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let filename = field.file_name().unwrap_or("upload.pdf").to_string();
        let bytes = field.bytes().await.map_err(|e| {
            AppError::BadRequest(format!(
                "Upload failed (max {} MB): {e}",
                service::MAX_UPLOAD_BYTES / 1024 / 1024
            ))
        })?;
        let import = service::start_import(&state, admin.id, &filename, bytes.to_vec()).await?;
        return Ok((
            StatusCode::ACCEPTED,
            Json(ImportResponse::from_row(
                import,
                state.config.bookeo.timezone,
            )),
        ));
    }
    Err(AppError::BadRequest(
        "No file in the upload (field \"file\")".into(),
    ))
}

pub async fn get_import(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ImportResponse>> {
    let row = service::get_import(&state, id).await?;
    Ok(Json(ImportResponse::from_row(
        row,
        state.config.bookeo.timezone,
    )))
}

pub async fn list_imports(State(state): State<AppState>) -> AppResult<Json<Vec<ImportResponse>>> {
    let tz = state.config.bookeo.timezone;
    let rows = service::recent_imports(&state).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| ImportResponse::from_row(r, tz))
            .collect(),
    ))
}

/// All players, best first (total points across all games).
pub async fn leaderboard(State(state): State<AppState>) -> AppResult<Json<Vec<Standing>>> {
    let tz = state.config.bookeo.timezone;
    let rows = service::leaderboard(&state).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Standing::from_row(r, tz))
            .collect(),
    ))
}
