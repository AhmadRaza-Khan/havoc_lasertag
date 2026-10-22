//! Webhooks: Bookeo calls when something changes on its side, and SkipCash reports payment
//! results (handled in `modules::payments`). For now: a booking canceled (or
//! deleted) in Bookeo, e.g. by staff in Bookeo's dashboard, is canceled here too.
//!
//! Bookeo signs every call (see `signature`) and retries until it gets a 2xx within 5 s, in
//! strict order per API key — so a message we can't use is still answered 2xx, or it would
//! hold up every message after it. The webhook is registered at startup when
//! `BOOKEO_WEBHOOK_URL` is set.

mod handlers;
mod signature;

use std::time::Duration;

use axum::{Router, routing::post};

use crate::{
    integrations::bookeo::{BookeoError, NewWebhook},
    state::AppState,
};

/// Where the "booking canceled" webhook is served (under `/api/webhooks`).
pub const BOOKINGS_DELETED_PATH: &str = "/api/webhooks/bookeo/bookings-deleted";

/// Public routes (Bookeo authenticates with a signature, not a token); mounted at `/webhooks`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bookeo/bookings-deleted", post(handlers::bookings_deleted))
        // SkipCash payment results (verified with the SkipCash webhook key).
        .route(
            "/skipcash",
            post(crate::modules::payments::skipcash_webhook),
        )
}

/// Adds our "bookings deleted" webhook to Bookeo if it isn't there yet, then reports if
/// Bookeo has blocked or delayed delivery. Never removes or replaces anything: other
/// webhooks on this API key (other domains/types) are left alone, and if a "bookings deleted"
/// webhook already points somewhere else we keep it and log an error (Bookeo allows one per
/// domain and type, and creating ours would replace it). Retries for a while if Bookeo is
/// unreachable at startup.
pub async fn register(state: AppState) {
    let Some(url) = state.config.bookeo.webhook_url.clone() else {
        tracing::info!("BOOKEO_WEBHOOK_URL is not set: bookings canceled in Bookeo aren't synced");
        return;
    };
    if !url
        .split('?')
        .next()
        .unwrap_or_default()
        .ends_with(BOOKINGS_DELETED_PATH)
    {
        tracing::warn!(%url, "BOOKEO_WEBHOOK_URL should end with {BOOKINGS_DELETED_PATH}");
    }

    let mut delay = Duration::from_secs(30);
    for attempt in 1..=6 {
        match try_register(&state, &url).await {
            Ok(()) => {
                report_status(&state).await;
                return;
            }
            Err(e) if attempt < 6 => {
                tracing::warn!(
                    "registering bookeo webhook failed (attempt {attempt}), retrying: {e}"
                );
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
            Err(e) => tracing::error!("registering bookeo webhook failed: {e}"),
        }
    }
}

async fn try_register(state: &AppState, url: &str) -> Result<(), BookeoError> {
    let existing = state.bookeo.webhooks().await?;
    match existing
        .iter()
        .find(|h| h.domain == "bookings" && h.kind == "deleted")
    {
        Some(h) if h.url == url => {
            tracing::info!(%url, "bookeo webhook already registered (bookings deleted)");
        }
        Some(h) => {
            tracing::error!(
                ours = %url,
                theirs = %h.url,
                "another \"bookings deleted\" webhook is registered on this Bookeo API key; \
                 left it in place, so bookings canceled in Bookeo aren't synced here"
            );
        }
        None => {
            let webhook = NewWebhook {
                url,
                domain: "bookings",
                kind: "deleted",
            };
            state.bookeo.create_webhook(&webhook).await?;
            tracing::info!(%url, "bookeo webhook added (bookings deleted)");
        }
    }
    Ok(())
}

/// Logs any webhook on this API key that Bookeo has blocked or is backing off from.
async fn report_status(state: &AppState) {
    match state.bookeo.webhooks().await {
        Ok(hooks) => {
            for h in hooks {
                if let Some(at) = &h.blocked_time {
                    tracing::error!(url = %h.url, domain = %h.domain, kind = %h.kind,
                        reason = h.blocked_reason.as_deref().unwrap_or(""), "bookeo webhook blocked at {at}");
                } else if let Some(until) = &h.no_delivery_before {
                    tracing::warn!(url = %h.url, reason = h.delivery_delay_reason.as_deref().unwrap_or(""),
                        "bookeo is delaying webhook deliveries until {until}");
                }
            }
        }
        Err(e) => tracing::warn!("could not read bookeo webhooks: {e}"),
    }
}
