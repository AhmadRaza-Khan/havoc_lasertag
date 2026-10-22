//! Background loop that keeps payments moving when nobody is watching: the customer closed
//! the app mid-payment, Bookeo was down when a payment came in, or time to pay ran out.

use std::time::Duration;

use super::{repository, service};
use crate::state::AppState;

const INTERVAL: Duration = Duration::from_secs(60);
/// Per run, per task. SkipCash is asked about at most this many payments a minute.
const BATCH: i64 = 50;

pub async fn run(state: AppState) {
    let mut tick = tokio::time::interval(INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        run_once(&state).await;
    }
}

pub async fn run_once(state: &AppState) {
    // 1. Payments made with the app closed.
    match repository::open_links(&state.db, BATCH).await {
        Ok(links) => {
            for p in links {
                if let Err(e) = service::poll_link(state, &p).await {
                    tracing::warn!(payment_id = %p.id, "could not check payment: {e}");
                }
            }
        }
        Err(e) => tracing::error!("listing open payment links: {e}"),
    }

    // 1b. Payments we stopped using, which SkipCash keeps payable for up to an hour.
    match repository::recently_closed(&state.db, BATCH).await {
        Ok(links) => {
            for p in links {
                if let Err(e) = service::poll_closed(state, &p).await {
                    tracing::warn!(payment_id = %p.id, "could not check closed payment: {e}");
                }
            }
        }
        Err(e) => tracing::error!("listing recently closed payments: {e}"),
    }

    // 2. Paid but not yet in Bookeo.
    match repository::unsynced(&state.db, BATCH).await {
        Ok(ids) => {
            for id in ids {
                if let Err(e) = service::record_in_bookeo(state, id).await {
                    tracing::warn!(payment_id = %id, "could not record payment in bookeo: {e}");
                }
            }
        }
        Err(e) => tracing::error!("listing unsynced payments: {e}"),
    }

    // 3. Time to pay ran out.
    match repository::overdue_bookings(&state.db, BATCH).await {
        Ok(ids) => {
            for id in ids {
                if let Err(e) = service::expire(state, id).await {
                    tracing::error!(booking_id = %id, "could not cancel unpaid booking: {e}");
                }
            }
        }
        Err(e) => tracing::error!("listing overdue bookings: {e}"),
    }

    // 4. Bookings canceled elsewhere (staff, Bookeo) must not stay payable.
    match repository::open_links_of_canceled_bookings(&state.db, BATCH).await {
        Ok(links) => {
            for p in links {
                if let Err(e) = service::close_for_canceled_booking(state, &p).await {
                    tracing::warn!(payment_id = %p.id, "could not close payment: {e}");
                }
            }
        }
        Err(e) => tracing::error!("listing links of canceled bookings: {e}"),
    }
}
