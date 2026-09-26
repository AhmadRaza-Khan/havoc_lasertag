use axum::{
    Json,
    extract::{Path, Query, State},
};

use super::{
    dto::{Game, SlotsQuery, SlotsResponse},
    service,
};
use crate::{error::AppResult, state::AppState};

pub async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Game>>> {
    Ok(Json(service::list(&state).await?))
}

pub async fn slots(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    Query(query): Query<SlotsQuery>,
) -> AppResult<Json<SlotsResponse>> {
    Ok(Json(service::slots(&state, &product_id, query).await?))
}
