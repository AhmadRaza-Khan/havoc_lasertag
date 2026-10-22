use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::{
    dto::{
        AdminListQuery, CreateBookingRequest, ListQuery, OptionInput, PlayerSummary,
        UpdateBookingRequest, validate_group,
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
            // Staff see app bookings in the admin panel; no Bookeo staff emails.
            Actor::Player => RequestOptions {
                notify_users: false,
                notify_customer: customer_has_email,
                backend: false,
            },
            // Staff made the change themselves; act as a manager.
            Actor::Admin => RequestOptions {
                notify_users: false,
                notify_customer: customer_has_email,
                backend: true,
            },
            // The customer gets our own email explaining why.
            Actor::System => RequestOptions {
                notify_users: false,
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
    let options = resolve_options(&product, &req.options, actor)?;

    let player = players::find_by_id(&state.db, player_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Player not found".into()))?;
    // Checked before anything is booked in Bookeo.
    check_other_players(state, player_id, &req.other_player_ids).await?;
    let customer_id = ensure_bookeo_customer(state, player_id).await?;

    // With online payment on, our emails (payment due, receipt) replace Bookeo's confirmation:
    // an app booking isn't final until it's paid.
    let bookeo_emails =
        player.email.is_some() && !(matches!(actor, Actor::Player) && state.skipcash().is_some());

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
                options: options.clone(),
            },
            actor.options(bookeo_emails),
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
            options: &options,
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
    // Bookeo replaces the whole booking: re-send the current add-ons unless they're changing.
    let options = match &req.options {
        Some(inputs) => resolve_options(&product, inputs, actor)?,
        None => row.options.0.clone(),
    };
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
                options: options.clone(),
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
            options: Some(&options),
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

/// The booking was canceled (or deleted) in Bookeo, e.g. by staff in Bookeo's dashboard:
/// mark ours canceled too. Bookeo already freed the seats, so nothing is sent back to it; an
/// open payment link is closed by the payments worker. Returns the booking if this changed
/// it (`None`: not one of ours, or already canceled — Bookeo also reports our own cancels).
pub async fn canceled_in_bookeo(
    state: &AppState,
    booking_number: &str,
) -> AppResult<Option<BookingRow>> {
    let row = repository::cancel_by_booking_number(&state.db, booking_number).await?;
    if let Some(r) = &row {
        tracing::info!(booking_id = %r.id, booking_number, "booking canceled in bookeo");
    }
    Ok(row)
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
            // Staff may have changed the add-ons in Bookeo.
            options: booking.options.as_deref(),
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

/// Longest text accepted for a text add-on.
const MAX_OPTION_TEXT: usize = 500;

/// Checks chosen add-ons against the game's options and turns them into what Bookeo expects.
/// Players can't set staff-only options; required text options must be filled in.
fn resolve_options(
    product: &bookeo::Product,
    inputs: &[OptionInput],
    actor: Actor,
) -> AppResult<Vec<bookeo::BookingOption>> {
    let player = matches!(actor, Actor::Player);
    let bad = |m: String| AppError::BadRequest(m);
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for input in inputs {
        let id = input.id.trim();
        if !seen.insert(id.to_string()) {
            return Err(bad("An add-on was chosen twice".into()));
        }
        let value = input.value.trim();
        let usable = |c: &bookeo::OptionCommon| {
            if !c.enabled || (player && !c.shown_to_customers) {
                Err(bad(format!("“{}” isn't available", c.name.trim())))
            } else {
                Ok(())
            }
        };
        let option = |c: &bookeo::OptionCommon, value: String| bookeo::BookingOption {
            id: Some(c.id.clone()),
            name: Some(c.name.trim().to_string()),
            value,
        };

        if let Some(o) = product.choice_options.iter().find(|o| o.common.id == id) {
            usable(&o.common)?;
            if value.is_empty() {
                continue;
            }
            // Bookeo wants the chosen value's name; accept its id (or name) from clients.
            let v = o
                .values
                .iter()
                .find(|v| v.id == value || v.name.trim() == value)
                .ok_or_else(|| {
                    bad(format!(
                        "Pick one of the choices for “{}”",
                        o.common.name.trim()
                    ))
                })?;
            out.push(option(&o.common, v.name.clone()));
        } else if let Some(o) = product.number_options.iter().find(|o| o.common.id == id) {
            usable(&o.common)?;
            let n: i32 = value
                .parse()
                .ok()
                .filter(|n| (o.min_value..=o.max_value).contains(n))
                .ok_or_else(|| {
                    bad(format!(
                        "“{}” must be a number from {} to {}",
                        o.common.name.trim(),
                        o.min_value,
                        o.max_value
                    ))
                })?;
            out.push(option(&o.common, n.to_string()));
        } else if let Some(o) = product.on_off_options.iter().find(|o| o.common.id == id) {
            usable(&o.common)?;
            let on = match value {
                "true" => true,
                "false" => false,
                _ => return Err(bad(format!("“{}” must be on or off", o.common.name.trim()))),
            };
            out.push(option(&o.common, on.to_string()));
        } else if let Some(o) = product.text_options.iter().find(|o| o.common.id == id) {
            usable(&o.common)?;
            if value.chars().count() > MAX_OPTION_TEXT {
                return Err(bad(format!(
                    "“{}” can be at most {MAX_OPTION_TEXT} characters",
                    o.common.name.trim()
                )));
            }
            if !value.is_empty() {
                out.push(option(&o.common, value.to_string()));
            }
        } else {
            return Err(bad("One of the add-ons isn't offered for this game".into()));
        }
    }

    for o in &product.text_options {
        let applies = o.common.enabled && (!player || o.common.shown_to_customers);
        if applies
            && o.required
            && !out
                .iter()
                .any(|b| b.id.as_deref() == Some(o.common.id.as_str()))
        {
            return Err(bad(format!("“{}” is required", o.common.name.trim())));
        }
    }
    Ok(out)
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

    /// Shaped like the client's real Bookeo product (global options get product-specific ids).
    fn product_with_addons() -> bookeo::Product {
        serde_json::from_value(serde_json::json!({
            "productId": "P1", "name": "Open Session", "type": "fixed",
            "choiceOptions": [{
                "id": "P1_UPG", "name": "UPGRADE YOUR BATTLE", "index": 0, "description": "<p>Upgrades</p>",
                "shownToCustomers": true, "enabled": true,
                "values": [
                    {"id": "P1_UPG_FULL", "name": "Full Tactical Upgrade", "description": ""},
                    {"id": "P1_UPG_NONE", "name": "NO UPGRADE ", "description": ""}
                ]
            }],
            "onOffOptions": [{
                "id": "P1_SND", "name": "Search & Destroy Mission", "index": 1,
                "shownToCustomers": true, "enabled": true, "defaultState": false
            }],
            "numberOptions": [{
                "id": "P1_STAFF", "name": "Staff note count", "index": 2,
                "shownToCustomers": false, "enabled": true, "minValue": 0, "maxValue": 5, "defaultValue": 0
            }],
            "textOptions": [{
                "id": "P1_OLD", "name": "Retired", "index": 3,
                "shownToCustomers": true, "enabled": false, "required": true
            }]
        }))
        .unwrap()
    }

    fn input(id: &str, value: &str) -> OptionInput {
        OptionInput {
            id: id.into(),
            value: value.into(),
        }
    }

    #[test]
    fn resolves_addons_for_bookeo() {
        let p = product_with_addons();
        let out = resolve_options(
            &p,
            &[input("P1_UPG", "P1_UPG_FULL"), input("P1_SND", "true")],
            Actor::Player,
        )
        .unwrap();
        assert_eq!(out.len(), 2);
        // Bookeo wants the chosen value's name, not its id.
        assert_eq!(
            (out[0].id.as_deref(), out[0].value.as_str()),
            (Some("P1_UPG"), "Full Tactical Upgrade")
        );
        assert_eq!(out[1].value, "true");
        // A value's name works too; an empty choice is simply left out.
        assert_eq!(
            resolve_options(&p, &[input("P1_UPG", "NO UPGRADE")], Actor::Admin).unwrap()[0].value,
            "NO UPGRADE "
        );
        assert!(
            resolve_options(&p, &[input("P1_UPG", "")], Actor::Admin)
                .unwrap()
                .is_empty()
        );
        assert!(
            resolve_options(&p, &[], Actor::Player).unwrap().is_empty(),
            "disabled required text is ignored"
        );
    }

    #[test]
    fn rejects_bad_addons() {
        let p = product_with_addons();
        let err = |inputs: &[OptionInput], actor| {
            resolve_options(&p, inputs, actor).unwrap_err().to_string()
        };
        assert!(err(&[input("P1_UPG", "nope")], Actor::Player).contains("Pick one"));
        assert!(err(&[input("P1_SND", "yes")], Actor::Player).contains("on or off"));
        assert!(err(&[input("OTHER_GAME_OPT", "true")], Actor::Player).contains("isn't offered"));
        assert!(
            err(
                &[input("P1_SND", "true"), input("P1_SND", "false")],
                Actor::Player
            )
            .contains("twice")
        );
        assert!(
            err(&[input("P1_OLD", "x")], Actor::Admin).contains("isn't available"),
            "disabled"
        );
        // Staff-only options: staff yes (within range), players no.
        assert!(err(&[input("P1_STAFF", "2")], Actor::Player).contains("isn't available"));
        assert_eq!(
            resolve_options(&p, &[input("P1_STAFF", "2")], Actor::Admin).unwrap()[0].value,
            "2"
        );
        assert!(err(&[input("P1_STAFF", "9")], Actor::Admin).contains("from 0 to 5"));
    }

    #[test]
    fn actor_options() {
        let p = Actor::Player.options(true);
        assert!(!p.notify_users && p.notify_customer && !p.backend);
        let a = Actor::Admin.options(false);
        assert!(!a.notify_users && !a.notify_customer && a.backend);
        let s = Actor::System.options(true);
        assert!(!s.notify_users && !s.notify_customer && s.backend);
    }
}
