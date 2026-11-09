use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::{
    error::{AppError, AppResult},
    modules::{games::dto::game_options, payments::normalize_amount},
    shared::image::sniff,
    state::AppState,
};

/// Largest accepted picture. The admin panel resizes before uploading.
pub const MAX_IMAGE_BYTES: usize = 900 * 1024;

/// The part of a Bookeo option id that's the same on every game: global add-ons appear on each
/// game as "<product id>_<option id>". Ids without a "_" are used whole.
pub fn addon_key(option_id: &str) -> &str {
    option_id.rsplit('_').next().unwrap_or(option_id)
}

/// An add-on as admins see it in Settings: one row per add-on, whichever games offer it.
#[derive(Debug, Serialize)]
pub struct Addon {
    pub key: String,
    pub name: String,
    /// `choice`, `number`, `on_off` or `text`.
    pub kind: &'static str,
    pub description: Option<String>,
    /// Only staff may set it (not offered in the mobile app).
    pub staff_only: bool,
    /// Names of the games that offer it.
    pub games: Vec<String>,
    pub image_url: Option<String>,
    pub image_updated_at: Option<DateTime<Utc>>,
    /// Set by admins, e.g. "50.00". For reference: Bookeo calculates what a booking costs.
    pub price: Option<String>,
}

/// Every add-on Bookeo offers on any game, in display order, with its picture.
pub async fn list(state: &AppState) -> AppResult<Vec<Addon>> {
    let products = state.bookeo.products().await?;
    let images = stored(&state.db).await?;
    let prices = prices(&state.db).await?;
    let mut out: Vec<Addon> = Vec::new();
    for p in products.iter() {
        for o in game_options(p) {
            let key = addon_key(&o.id).to_string();
            let game = p.name.trim().to_string();
            match out.iter_mut().find(|a| a.key == key) {
                Some(a) => {
                    if !a.games.contains(&game) {
                        a.games.push(game);
                    }
                }
                None => {
                    let updated = images.get(&key).copied();
                    out.push(Addon {
                        price: prices.get(&key).cloned(),
                        image_url: updated.map(|t| url(&key, t)),
                        image_updated_at: updated,
                        key,
                        name: o.name,
                        kind: o.kind,
                        description: o.description,
                        staff_only: o.staff_only,
                        games: vec![game],
                    });
                }
            }
        }
    }
    Ok(out)
}

/// Picture URLs by add-on key, for the add-ons that have one.
pub async fn image_urls(db: &PgPool) -> AppResult<HashMap<String, String>> {
    Ok(stored(db)
        .await?
        .into_iter()
        .map(|(k, t)| {
            let u = url(&k, t);
            (k, u)
        })
        .collect())
}

pub async fn set_image(state: &AppState, key: &str, bytes: &[u8]) -> AppResult<Vec<Addon>> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(AppError::BadRequest(
            "The image is too large (max 900 KB)".into(),
        ));
    }
    let content_type =
        sniff(bytes).ok_or_else(|| AppError::BadRequest("Use a JPEG, PNG or WebP image".into()))?;
    ensure_exists(state, key).await?;
    sqlx::query(
        "INSERT INTO addon_images (addon_key, content_type, data) VALUES ($1, $2, $3)
         ON CONFLICT (addon_key) DO UPDATE SET content_type = $2, data = $3, updated_at = now()",
    )
    .bind(key)
    .bind(content_type)
    .bind(bytes)
    .execute(&state.db)
    .await?;
    tracing::info!(addon = key, bytes = bytes.len(), "add-on picture updated");
    list(state).await
}

pub async fn remove_image(state: &AppState, key: &str) -> AppResult<Vec<Addon>> {
    sqlx::query("DELETE FROM addon_images WHERE addon_key = $1")
        .bind(key)
        .execute(&state.db)
        .await?;
    tracing::info!(addon = key, "add-on picture removed");
    list(state).await
}

/// Add-on prices by add-on key, for the add-ons that have one.
pub async fn prices(db: &PgPool) -> AppResult<HashMap<String, String>> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT addon_key, amount::text FROM addon_prices")
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().collect())
}

pub async fn set_price(state: &AppState, key: &str, amount: &str) -> AppResult<Vec<Addon>> {
    let amount = parse_price(amount)?;
    ensure_exists(state, key).await?;
    sqlx::query(
        "INSERT INTO addon_prices (addon_key, amount) VALUES ($1, $2::numeric)
         ON CONFLICT (addon_key) DO UPDATE SET amount = $2::numeric, updated_at = now()",
    )
    .bind(key)
    .bind(&amount)
    .execute(&state.db)
    .await?;
    tracing::info!(addon = key, %amount, "add-on price updated");
    list(state).await
}

pub async fn remove_price(state: &AppState, key: &str) -> AppResult<Vec<Addon>> {
    sqlx::query("DELETE FROM addon_prices WHERE addon_key = $1")
        .bind(key)
        .execute(&state.db)
        .await?;
    tracing::info!(addon = key, "add-on price removed");
    list(state).await
}

/// "50" / "49.5" → "50.00" / "49.50"; at most 2 decimals and what the column can hold.
fn parse_price(amount: &str) -> AppResult<String> {
    normalize_amount(amount)
        .filter(|a| a.len() <= 12)
        .ok_or_else(|| AppError::BadRequest("Enter a price like 50 or 49.50".into()))
}

pub async fn image(db: &PgPool, key: &str) -> AppResult<(String, Vec<u8>)> {
    sqlx::query_as("SELECT content_type, data FROM addon_images WHERE addon_key = $1")
        .bind(key)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("No image".into()))
}

/// Pictures can only be set for add-ons Bookeo actually offers.
async fn ensure_exists(state: &AppState, key: &str) -> AppResult<()> {
    let products = state.bookeo.products().await?;
    let known = products
        .iter()
        .flat_map(game_options)
        .any(|o| addon_key(&o.id) == key);
    if known {
        Ok(())
    } else {
        Err(AppError::NotFound("Add-on not found".into()))
    }
}

async fn stored(db: &PgPool) -> AppResult<HashMap<String, DateTime<Utc>>> {
    let rows: Vec<(String, DateTime<Utc>)> =
        sqlx::query_as("SELECT addon_key, updated_at FROM addon_images")
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().collect())
}

/// Versioned, so caches see a new picture at once.
fn url(key: &str, updated_at: DateTime<Utc>) -> String {
    format!(
        "/api/addon-images/{key}?v={}",
        updated_at.timestamp_millis()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prices() {
        assert_eq!(parse_price(" 50 ").unwrap(), "50.00");
        assert_eq!(parse_price("49.5").unwrap(), "49.50");
        assert_eq!(parse_price("0").unwrap(), "0.00");
        for bad in ["", "abc", "-5", "1.234", "123456789012"] {
            assert!(parse_price(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn keys_are_shared_across_games() {
        assert_eq!(
            addon_key("42552TMJECE1A0B5756202_CLAHMUNU9K6K"),
            "CLAHMUNU9K6K"
        );
        assert_eq!(
            addon_key("425523XPJWT1A15A0F7F0F_CLAHMUNU9K6K"),
            "CLAHMUNU9K6K"
        );
        assert_eq!(addon_key("PLAIN"), "PLAIN");
    }
}
