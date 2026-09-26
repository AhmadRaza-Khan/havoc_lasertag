use chrono::Utc;

use super::{
    dto::{AvailableSlot, Game, SlotsQuery, SlotsResponse},
    range,
};
use crate::{
    error::{AppError, AppResult},
    integrations::bookeo::Product,
    state::AppState,
};

/// Whether players can book this product through the app. Only "fixed" products (scheduled
/// sessions with seats) are supported; courses and flexible-time products are not.
fn is_bookable(p: &Product) -> bool {
    p.product_type == "fixed"
        && p.api_bookings_allowed != Some(false)
        && p.shown_to_customers != Some(false)
}

pub async fn list(state: &AppState) -> AppResult<Vec<Game>> {
    let products = state.bookeo.products().await?;
    let mut games: Vec<Game> = products
        .iter()
        .filter(|p| is_bookable(p))
        .map(Game::from)
        .collect();
    games.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(games)
}

/// Looks up a product players are allowed to book. Used by the bookings module too.
pub async fn bookable_product(state: &AppState, product_id: &str) -> AppResult<Product> {
    state
        .bookeo
        .products()
        .await?
        .iter()
        .find(|p| p.product_id == product_id && is_bookable(p))
        .cloned()
        .ok_or_else(|| AppError::NotFound("Game not found".into()))
}

pub async fn slots(
    state: &AppState,
    product_id: &str,
    query: SlotsQuery,
) -> AppResult<SlotsResponse> {
    let tz = state.config.bookeo.timezone;
    let window = range::resolve(query.from, query.to, tz, Utc::now())?;
    let product = bookable_product(state, product_id).await?;

    let slots = state
        .bookeo
        .slots(&product.product_id, &window.start, &window.end)
        .await?;

    let mut available: Vec<AvailableSlot> = slots
        .into_iter()
        .filter(|s| s.seats_available() > 0 && s.start_time >= window.start)
        .map(|s| AvailableSlot {
            seats_available: s.seats_available(),
            event_id: s.event_id,
            start_time: s.start_time,
            end_time: s.end_time,
        })
        .collect();
    available.sort_by_key(|s| s.start_time);

    Ok(SlotsResponse {
        product_id: product.product_id,
        game_name: product.name.trim().to_string(),
        timezone: tz.name().to_string(),
        from: window.from,
        to: window.to,
        slots: available,
    })
}
