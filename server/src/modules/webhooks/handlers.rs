use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;

use super::signature::{self, Signed};
use crate::{
    error::{AppError, AppResult},
    integrations::bookeo::WebhookMessage,
    modules::bookings,
    state::AppState,
};

/// `POST /api/webhooks/bookeo/bookings-deleted`: Bookeo reports a booking canceled or
/// deleted. Signed calls are always answered 2xx unless we failed to save (then Bookeo
/// retries); unsigned ones get 401.
pub async fn bookings_deleted(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<StatusCode> {
    let Some(url) = state.config.bookeo.webhook_url.as_deref() else {
        return Err(AppError::NotFound("Not found".into()));
    };
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
    };
    let message_id = header("x-bookeo-messageid");

    let signed = Signed {
        timestamp: header("x-bookeo-timestamp"),
        message_id,
        signature: header("x-bookeo-signature"),
        body: &body,
    };
    if let Err(why) = signature::verify(
        &state.config.bookeo.secret_key,
        url,
        &signed,
        Utc::now().timestamp_millis(),
    ) {
        tracing::warn!(?why, message_id, "rejected bookeo webhook call");
        return Err(AppError::Unauthorized("Invalid signature".into()));
    }
    if header("x-bookeo-previousmessagelost") == "true" {
        tracing::warn!(
            message_id,
            "bookeo dropped earlier webhook messages: a booking canceled in Bookeo may still look active here"
        );
    }

    let message: WebhookMessage = match serde_json::from_slice(&body) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(message_id, "unreadable bookeo webhook message: {e}");
            return Ok(StatusCode::NO_CONTENT);
        }
    };
    let booking_number = message.item_id.or_else(|| {
        message
            .item
            .as_ref()
            .and_then(|i| i.get("bookingNumber"))
            .and_then(|n| n.as_str())
            .map(str::to_owned)
    });
    let Some(booking_number) = booking_number.filter(|n| !n.trim().is_empty()) else {
        tracing::warn!(
            message_id,
            "bookeo webhook message without a booking number"
        );
        return Ok(StatusCode::NO_CONTENT);
    };

    // A database error propagates as 500, so Bookeo delivers the message again.
    let changed = bookings::canceled_in_bookeo(&state, booking_number.trim()).await?;
    if !changed {
        tracing::debug!(
            message_id,
            booking_number,
            "bookeo cancel: not ours or already canceled"
        );
    }
    Ok(StatusCode::NO_CONTENT)
}
