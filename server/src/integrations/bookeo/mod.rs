//! Bookeo API v2 client (https://www.bookeo.com/api). Bookeo is the source of truth for
//! games (products), availability and bookings.

mod client;
mod types;

pub use client::{BookeoClient, BookeoError, RequestOptions};
pub use types::*;
