use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::{
    dto::{
        AdminListQuery, CreateBookingRequest, ListQuery, PlayerSummary, UpdateBookingRequest,
        validate_group,
    },
    model::{BookingRow, BookingWithPlayer, status_of},
    repository::{self, AdminFilter, BookingChanges, NewBookingRow},
};
use crate::{
    error::{AppError, AppResult},
    integrations::bookeo::{
        self, NewBooking, NewCustomer, Participants, PeopleNumber, PhoneNumber, RequestOptions,
    },
    modules::{
        games::{dto::people_limits, service::bookable_product},
        players::repository as players,
    },
    shared::pagination::{PageParams, Paginated, like_pattern},
    state::AppState,
};

/// Who is making a change. Decides Bookeo notifications and whether limits are relaxed.
#[derive(Debug, Clone, Copy)]
pub enum Actor {
    /// A player in the mobile app, acting on their own bookings.
    Player,
    /// Staff in the admin panel.
    Admin,
    /// The server itself, e.g. canceling a booking that wasn't paid in time.
    System,
}

impl Actor {
    fn options(self, customer_has_email: bool) -> RequestOptions {
        match self {
            // Staff should hear about bookings made in the app.
            Actor::Player => RequestOptions {
                notify_users: true,
                notify_customer: customer_has_email,
                backend: false,
            },
            // Staff made the change themselves; act as a manager.
            Actor::Admin => RequestOptions {
                notify_users: false,
                notify_customer: customer_has_email,
                backend: true,
            },
            // Staff hear about it; the customer gets our own email explaining why.
            Actor::System => RequestOptions {
                notify_users: true,
                notify_customer: false,
                backend: true,
            },
        }
    }

    fn source(self) -> &'static str {
        match self {
            Actor::Player => "Havoc app",
            Actor::Admin | Actor::System => "Havoc admin",
        }
    }
}

// ---------------------------------------------------------------------------
// Create / update / cancel
// ---------------------------------------------------------------------------

pub async fn create(
    state: &AppState,
    player_id: Uuid,
    req: CreateBookingRequest,
    actor: Actor,
) -> AppResult<BookingRow> {
    let req = req.validated()?;
    let product = bookable_product(state, &req.product_id).await?;
    let participants = participants_for(&product, req.num_people)?;

    let player = players::find_by_id(&state.db, player_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Player not found".into()))?;
    // Checked before anything is booked in Bookeo.
    check_other_players(state, player_id, &req.other_player_ids).await?;
    let customer_id = ensure_bookeo_customer(state, player_id).await?;

    let id = Uuid::new_v4();
    let booking = state
        .bookeo
        .create_booking(
            &NewBooking {
                product_id: product.product_id.clone(),
                event_id: req.event_id.clone(),
                customer_id,
                external_ref: id.to_string(),
                participants,
                source: actor.source(),
            },
            actor.options(player.email.is_some()),
        )
        .await?;
    tracing::info!(%player_id, ?actor, booking_number = %booking.booking_number, "bookeo booking created");

    let booking = with_times(state, booking).await?;
    let (start, end) = times(&booking)?;
    let price = booking.price.as_ref().and_then(|p| p.total_gross.as_ref());

    let row = repository::insert(
        &state.db,
        &NewBookingRow {
            id,
            player_id,
            bookeo_booking_number: &booking.booking_number,
            product_id: &product.product_id,
            product_name: product.name.trim(),
            event_id: &req.event_id,
            start_time: start,
            end_time: end,
            num_people: req.num_people,
            other_players: &req.other_player_ids,
            status: status_of(&booking),
            price_amount: price.map(|m| m.amount.as_str()),
            price_currency: price.map(|m| m.currency.as_str()),
        },
    )
    .await
    .inspect_err(|e| {
        // The booking exists in Bookeo but not locally — must be reconciled by hand.
        tracing::error!(
            %player_id,
            booking_number = %booking.booking_number,
            "failed to save booking created in bookeo: {e}"
        );
    })?;

    if matches!(actor, Actor::Player) {
        // Online payment applies to app bookings; staff collect for their own at the venue.
        if let Err(e) = crate::modules::payments::on_booking_created(state, row.id).await {
            tracing::error!(booking_id = %row.id, "could not set up payment: {e}");
        }
        return find(state, row.id, None).await;
    }
    Ok(row)
}

/// Moves a booking to another slot of the same game and/or changes the group size.
pub async fn update(
    state: &AppState,
    id: Uuid,
    req: UpdateBookingRequest,
    actor: Actor,
) -> AppResult<BookingRow> {
    let req = req.validated()?;
    let row = find(state, id, None).await?;
    if row.status == "canceled" {
        return Err(AppError::Conflict(
            "Canceled bookings can't be changed".into(),
        ));
    }

    match &req.other_player_ids {
        Some(others) => check_other_players(state, row.player_id, others).await?,
        // Keeping the current other players: the new group size must still fit them.
        None => validate_group(req.num_people, row.other_players.len())?,
    }

    let product = bookable_product(state, &row.product_id).await?;
    let participants = participants_for(&product, req.num_people)?;
    let customer_id = ensure_bookeo_customer(state, row.player_id).await?;
    let has_email = players::find_by_id(&state.db, row.player_id)
        .await?
        .is_some_and(|p| p.email.is_some());

    let booking = state
        .bookeo
        .update_booking(
            &row.bookeo_booking_number,
            &NewBooking {
                product_id: row.product_id.clone(),
                event_id: req.event_id.clone(),
                customer_id,
                external_ref: row.id.to_string(),
                participants,
                source: actor.source(),
            },
            actor.options(has_email),
        )
        .await?;
    tracing::info!(booking_id = %id, ?actor, "bookeo booking updated");

    let booking = with_times(state, booking).await?;
    let (start, end) = times(&booking)?;
    let price = booking.price.as_ref().and_then(|p| p.total_gross.as_ref());

    Ok(repository::apply_changes(
        &state.db,
        id,
        &BookingChanges {
            event_id: Some(booking.event_id.as_deref().unwrap_or(&req.event_id)),
            num_people: Some(booking.num_people().unwrap_or(req.num_people)),
            other_players: req.other_player_ids.as_deref(),
            status: status_of(&booking),
            start_time: start,
            end_time: end,
            price_amount: price.map(|m| m.amount.as_str()),
            price_currency: price.map(|m| m.currency.as_str()),
        },
    )
    .await?)
}

/// Cancels in Bookeo (frees the seats) and keeps the row as `canceled` for history.
pub async fn cancel(state: &AppState, id: Uuid, actor: Actor) -> AppResult<BookingRow> {
    let row = find(state, id, None).await?;
    if row.status == "canceled" {
        return Err(AppError::Conflict("Booking is already canceled".into()));
    }
    let has_email = players::find_by_id(&state.db, row.player_id)
        .await?
        .is_some_and(|p| p.email.is_some());

    state
        .bookeo
        .cancel_booking(&row.bookeo_booking_number, actor.options(has_email))
        .await?;
    tracing::info!(booking_id = %id, ?actor, "bookeo booking canceled");

    Ok(repository::set_status(&state.db, id, "canceled").await?)
}

// ---------------------------------------------------------------------------
// Read
// ---------------------------------------------------------------------------

pub async fn list_for_player(
    state: &AppState,
    player_id: Uuid,
    query: ListQuery,
) -> AppResult<Paginated<BookingRow>> {
    let params = PageParams {
        page: query.page,
        per_page: query.per_page,
    };
    let (rows, total) = repository::list_for_player(
        &state.db,
        player_id,
        query.when.map(|w| w.as_str()),
        params.per_page(),
        params.offset(),
    )
    .await?;
    Ok(Paginated::new(rows, total, &params))
}

pub async fn list_all(
    state: &AppState,
    query: AdminListQuery,
) -> AppResult<Paginated<BookingWithPlayer>> {
    let params = PageParams {
        page: query.page,
        per_page: query.per_page,
    };
    let pattern = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(like_pattern);

    let (rows, total) = repository::list_all(
        &state.db,
        &AdminFilter {
            pattern: pattern.as_deref(),
            player_id: query.player_id,
            when: query.when.map(|w| w.as_str()),
            status: query.status.map(|s| s.as_str()),
        },
        params.per_page(),
        params.offset(),
    )
    .await?;
    Ok(Paginated::new(rows, total, &params))
}

/// One booking, refreshed from Bookeo (staff may have accepted, canceled or moved it there).
/// With `player_id`, the booking must belong to that player. If Bookeo is unreachable, the
/// stored copy is returned.
pub async fn get(state: &AppState, id: Uuid, player_id: Option<Uuid>) -> AppResult<BookingRow> {
    let row = find(state, id, player_id).await?;

    let booking = match state.bookeo.booking(&row.bookeo_booking_number).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(booking_id = %row.id, "could not refresh booking from bookeo: {e}");
            return Ok(row);
        }
    };
    let (Some(start), Some(end)) = (booking.start_time, booking.end_time) else {
        return Ok(row);
    };
    let price = booking.price.as_ref().and_then(|p| p.total_gross.as_ref());

    Ok(repository::apply_changes(
        &state.db,
        row.id,
        &BookingChanges {
            event_id: booking.event_id.as_deref(),
            num_people: booking.num_people(),
            other_players: None,
            status: status_of(&booking),
            start_time: start.to_utc(),
            end_time: end.to_utc(),
            price_amount: price.map(|m| m.amount.as_str()),
            price_currency: price.map(|m| m.currency.as_str()),
        },
    )
    .await?)
}

pub async fn with_player(state: &AppState, id: Uuid) -> AppResult<BookingWithPlayer> {
    repository::find_with_player(&state.db, id)
        .await?
        .ok_or_else(not_found)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn find(state: &AppState, id: Uuid, player_id: Option<Uuid>) -> AppResult<BookingRow> {
    repository::find(&state.db, id, player_id)
        .await?
        .ok_or_else(not_found)
}

/// Other players must be existing players and can't include the booking's own player.
async fn check_other_players(state: &AppState, player_id: Uuid, others: &[Uuid]) -> AppResult<()> {
    if others.contains(&player_id) {
        return Err(AppError::BadRequest(
            "The booking's player is already in the group; don't add them as an other player"
                .into(),
        ));
    }
    let found = repository::player_summaries(&state.db, others).await?;
    if found.len() != others.len() {
        return Err(AppError::NotFound(
            "One of the other players doesn't exist".into(),
        ));
    }
    Ok(())
}

/// Name/playername of every other player referenced by `rows`, in one query.
pub async fn other_players_lookup<'a>(
    state: &AppState,
    rows: impl IntoIterator<Item = &'a BookingRow>,
) -> AppResult<HashMap<Uuid, PlayerSummary>> {
    let ids: Vec<Uuid> = rows
        .into_iter()
        .flat_map(|r| r.other_players.iter().copied())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    Ok(repository::player_summaries(&state.db, &ids)
        .await?
        .into_iter()
        .map(|(id, full_name, playername)| {
            (
                id,
                PlayerSummary {
                    id,
                    full_name,
                    playername,
                },
            )
        })
        .collect())
}

fn not_found() -> AppError {
    AppError::NotFound("Booking not found".into())
}

/// Checks the group size against the game's limits and builds Bookeo's participants.
fn participants_for(product: &bookeo::Product, num_people: i32) -> AppResult<Participants> {
    if let Some((min, max)) = people_limits(product)
        && !(min..=max).contains(&num_people)
    {
        return Err(AppError::BadRequest(format!(
            "This game is for {min}-{max} people"
        )));
    }
    let category = people_category(product).ok_or_else(|| {
        AppError::Internal(format!(
            "product {} has no people category",
            product.product_id
        ))
    })?;
    Ok(Participants {
        numbers: vec![PeopleNumber {
            people_category_id: category,
            number: num_people,
        }],
    })
}

/// The people category to book under: the one the product's limits or rates refer to.
fn people_category(product: &bookeo::Product) -> Option<String> {
    product
        .booking_limits
        .iter()
        .find_map(|l| l.people_category_id.clone())
        .or_else(|| {
            product
                .default_rates
                .iter()
                .find_map(|r| r.people_category_id.clone())
        })
}

/// Re-fetches the booking if Bookeo's write response left out its times.
async fn with_times(state: &AppState, booking: bookeo::Booking) -> AppResult<bookeo::Booking> {
    if booking.start_time.is_some() && booking.end_time.is_some() {
        return Ok(booking);
    }
    Ok(state.bookeo.booking(&booking.booking_number).await?)
}

fn times(
    booking: &bookeo::Booking,
) -> AppResult<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> {
    match (booking.start_time, booking.end_time) {
        (Some(s), Some(e)) => Ok((s.to_utc(), e.to_utc())),
        _ => Err(AppError::Internal(format!(
            "bookeo booking {} has no start/end time",
            booking.booking_number
        ))),
    }
}

/// Each player is mirrored as one Bookeo customer, created on their first booking.
async fn ensure_bookeo_customer(state: &AppState, player_id: Uuid) -> AppResult<String> {
    if let Some(id) = players::bookeo_customer_id(&state.db, player_id).await? {
        return Ok(id);
    }

    let player = players::find_by_id(&state.db, player_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Player not found".into()))?;

    let (first_name, last_name) = split_name(&player.full_name);
    let customer_id = state
        .bookeo
        .create_customer(&NewCustomer {
            first_name,
            last_name,
            email_address: player.email.clone(),
            phone_numbers: player
                .phone
                .iter()
                .map(|n| PhoneNumber {
                    number: n.clone(),
                    kind: "mobile",
                })
                .collect(),
        })
        .await?;
    tracing::info!(%player_id, %customer_id, "bookeo customer created");

    // If a concurrent request won the race, use its customer id.
    Ok(players::set_bookeo_customer_id(&state.db, player_id, &customer_id).await?)
}

/// "Jane van Doe" → ("Jane", "van Doe"); a single word is used for both.
fn split_name(full_name: &str) -> (String, String) {
    let full_name = full_name.trim();
    match full_name.split_once(char::is_whitespace) {
        Some((first, last)) => (first.to_string(), last.trim().to_string()),
        None => (full_name.to_string(), full_name.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_names() {
        assert_eq!(
            split_name(" Jane  van Doe "),
            ("Jane".into(), "van Doe".into())
        );
        assert_eq!(split_name("Madonna"), ("Madonna".into(), "Madonna".into()));
    }

    #[test]
    fn actor_options() {
        let p = Actor::Player.options(true);
        assert!(p.notify_users && p.notify_customer && !p.backend);
        let a = Actor::Admin.options(false);
        assert!(!a.notify_users && !a.notify_customer && a.backend);
    }
}
