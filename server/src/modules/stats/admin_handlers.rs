//! Admin panel: upload game reports, follow imports, see the general report.

use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use super::{
    dto::{GameRecord, ImportResponse, PageQuery, PlayerStats, Standing},
    repository, service,
};
use crate::{
    error::{AppError, AppResult},
    modules::auth::AuthUser,
    shared::pagination::{PageParams, Paginated},
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

/// One player's all-time totals (from the leaderboard). Players who haven't played have
/// zeros and no position.
pub async fn player_standing(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PlayerStats>> {
    let row = repository::standing(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Player not found".into()))?;
    Ok(Json(PlayerStats {
        standing: Standing::from_row(row, state.config.bookeo.timezone),
        details: service::player_details(&state, id).await?,
    }))
}

/// One player's game records with everything from the reports, newest first.
pub async fn player_games(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<PageQuery>,
) -> AppResult<Json<Paginated<GameRecord>>> {
    let params = PageParams {
        page: q.page,
        per_page: q.per_page,
    };
    let tz = state.config.bookeo.timezone;
    let page = service::player_records(&state, id, params).await?;
    Ok(Json(page.map(|r| GameRecord::from_row(r, tz))))
}
