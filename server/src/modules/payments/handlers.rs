use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use uuid::Uuid;

use super::service::{self, PaymentView, SettingsInput, SettingsResponse};
use crate::{error::AppResult, modules::player_auth::AuthPlayer, state::AppState};

// --- Admin ---

pub async fn get_settings(State(state): State<AppState>) -> AppResult<Json<SettingsResponse>> {
    Ok(Json(service::settings(&state).await?))
}

pub async fn put_settings(
    State(state): State<AppState>,
    Json(input): Json<SettingsInput>,
) -> AppResult<Json<SettingsResponse>> {
    Ok(Json(service::save_settings(&state, input).await?))
}

// --- Mobile app ---

#[derive(Debug, Deserialize)]
pub struct StartQuery {
    /// `ar` for QNB's Arabic payment page; English otherwise.
    pub lang: Option<String>,
}

/// Gets a QNB payment page to open in a WebView.
pub async fn start(
    State(state): State<AppState>,
    player: AuthPlayer,
    Path(id): Path<Uuid>,
    Query(q): Query<StartQuery>,
) -> AppResult<Json<PaymentView>> {
    let arabic = q.lang.as_deref().is_some_and(|l| l.starts_with("ar"));
    Ok(Json(service::start(&state, id, player.id, arabic).await?))
}

/// Payment state, confirmed with QNB. Poll while the payment page is open.
pub async fn status(
    State(state): State<AppState>,
    player: AuthPlayer,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PaymentView>> {
    Ok(Json(service::status(&state, id, player.id).await?))
}

/// The customer closed the payment page without paying. Cancels a "pay now" booking right
/// away; a "pay later" booking stays payable until its deadline.
pub async fn abandon(
    State(state): State<AppState>,
    player: AuthPlayer,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PaymentView>> {
    Ok(Json(service::abandon(&state, id, player.id).await?))
}
