use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
};
use serde::Deserialize;
use uuid::Uuid;

use super::{
    credentials::{self, CredentialsInput, CredentialsView, TestResult},
    service::{self, PaymentView, SettingsInput, SettingsResponse},
};
use crate::{
    error::{AppError, AppResult},
    integrations::skipcash::{WebhookPayload, verify_webhook},
    modules::player_auth::AuthPlayer,
    state::AppState,
};

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

/// SkipCash credentials (secrets are never returned, only whether they're saved).
pub async fn get_skipcash(State(state): State<AppState>) -> AppResult<Json<CredentialsView>> {
    Ok(Json(credentials::view(&state).await?))
}

pub async fn put_skipcash(
    State(state): State<AppState>,
    Json(input): Json<CredentialsInput>,
) -> AppResult<Json<CredentialsView>> {
    Ok(Json(credentials::save(&state, input).await?))
}

pub async fn delete_skipcash(State(state): State<AppState>) -> AppResult<Json<CredentialsView>> {
    Ok(Json(credentials::remove(&state).await?))
}

/// Tries credentials against SkipCash without saving them.
pub async fn test_skipcash(
    State(state): State<AppState>,
    Json(input): Json<CredentialsInput>,
) -> AppResult<Json<TestResult>> {
    Ok(Json(credentials::test(&state, input).await?))
}

// --- Mobile app ---

/// Optional body of `POST .../payment`.
#[derive(Debug, Default, Deserialize)]
pub struct StartPayment {
    /// How much to pay now: at least QAR 120 (or the rest, if less is left), at most the rest.
    /// Omit to pay the whole rest.
    pub amount: Option<String>,
}

/// Gets a SkipCash payment page to open in a WebView, for the whole rest or part of it. The
/// booking's player and its other players can all pay.
pub async fn start(
    State(state): State<AppState>,
    player: AuthPlayer,
    Path(id): Path<Uuid>,
    body: Bytes,
) -> AppResult<Json<PaymentView>> {
    let input: StartPayment = if body.iter().all(u8::is_ascii_whitespace) {
        StartPayment::default()
    } else {
        serde_json::from_slice(&body)
            .map_err(|e| AppError::BadRequest(format!("Invalid request body: {e}")))?
    };
    Ok(Json(
        service::start(&state, id, player.id, input.amount.as_deref()).await?,
    ))
}

/// Payment state, confirmed with SkipCash. Poll while the payment page is open.
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

// --- SkipCash ---

/// `POST /api/webhooks/skipcash`: SkipCash reports a payment's result (manual §4). Signed with
/// our webhook key; anything else is refused. Valid calls are answered 200 even when we ignore
/// them (SkipCash retries anything else).
pub async fn skipcash_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<StatusCode> {
    let Some(key) = state.skipcash().map(|c| c.webhook_key().to_string()) else {
        return Err(AppError::NotFound("Not found".into()));
    };
    let payload: WebhookPayload = serde_json::from_slice(&body)
        .map_err(|e| AppError::BadRequest(format!("Invalid webhook body: {e}")))?;
    let authorization = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !verify_webhook(&key, &payload, authorization) {
        tracing::warn!(payment_id = %payload.payment_id, "rejected skipcash webhook (bad signature)");
        return Err(AppError::Unauthorized("Invalid signature".into()));
    }
    // A database error propagates as 500, so SkipCash calls again later.
    service::on_webhook(&state, &payload).await?;
    Ok(StatusCode::OK)
}
