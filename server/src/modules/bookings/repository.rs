use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::model::{BookingRow, BookingWithPlayer};

/// Every query aliases `bookings` as `b` so the same column list works with joins.
const COLUMNS: &str = "b.id, b.player_id, b.bookeo_booking_number, b.product_id, b.product_name, \
    b.event_id, b.start_time, b.end_time, b.num_people, b.other_players, b.status, b.price_amount, b.price_currency, \
    b.payment_status, b.payment_due_at, b.created_at, b.updated_at";

const PLAYER_COLUMNS: &str = "p.full_name AS player_full_name, p.playername AS player_playername";

/// `$when` placeholder filter: NULL, 'upcoming' (not yet ended) or 'past' (ended).
fn when_filter(param: &str) -> String {
    format!(
        "({param}::text IS NULL
          OR ({param} = 'upcoming' AND b.end_time > now())
          OR ({param} = 'past' AND b.end_time <= now()))"
    )
}

/// Upcoming: soonest first. Otherwise: latest first.
fn order_by(when_param: &str) -> String {
    format!(
        "ORDER BY CASE WHEN {when_param} = 'upcoming' THEN b.start_time END ASC, b.start_time DESC"
    )
}

pub struct NewBookingRow<'a> {
    pub id: Uuid,
    pub player_id: Uuid,
    pub bookeo_booking_number: &'a str,
    pub product_id: &'a str,
    pub product_name: &'a str,
    pub event_id: &'a str,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub num_people: i32,
    pub other_players: &'a [Uuid],
    pub status: &'a str,
    pub price_amount: Option<&'a str>,
    pub price_currency: Option<&'a str>,
}

pub async fn insert(db: &PgPool, b: &NewBookingRow<'_>) -> sqlx::Result<BookingRow> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO bookings AS b (id, player_id, bookeo_booking_number, product_id, product_name,
             event_id, start_time, end_time, num_people, status, price_amount, price_currency,
             other_players)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         RETURNING {COLUMNS}"
    )))
    .bind(b.id)
    .bind(b.player_id)
    .bind(b.bookeo_booking_number)
    .bind(b.product_id)
    .bind(b.product_name)
    .bind(b.event_id)
    .bind(b.start_time)
    .bind(b.end_time)
    .bind(b.num_people)
    .bind(b.status)
    .bind(b.price_amount)
    .bind(b.price_currency)
    .bind(b.other_players)
    .fetch_one(db)
    .await
}

/// A player's own bookings.
pub async fn list_for_player(
    db: &PgPool,
    player_id: Uuid,
    when: Option<&str>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<(Vec<BookingRow>, i64)> {
    let filter = format!("b.player_id = $1 AND {}", when_filter("$2"));

    let items = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM bookings b WHERE {filter} {} LIMIT $3 OFFSET $4",
        order_by("$2")
    )))
    .bind(player_id)
    .bind(when)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM bookings b WHERE {filter}"
    )))
    .bind(player_id)
    .bind(when)
    .fetch_one(db)
    .await?;

    Ok((items, total))
}

pub struct AdminFilter<'a> {
    /// Escaped ILIKE pattern (see `like_pattern`).
    pub pattern: Option<&'a str>,
    pub player_id: Option<Uuid>,
    pub when: Option<&'a str>,
    pub status: Option<&'a str>,
}

/// All bookings with their players, for the admin panel.
pub async fn list_all(
    db: &PgPool,
    f: &AdminFilter<'_>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<(Vec<BookingWithPlayer>, i64)> {
    let filter = format!(
        "($1::text IS NULL OR p.full_name ILIKE $1 OR p.playername ILIKE $1
              OR b.bookeo_booking_number ILIKE $1)
         AND ($2::uuid IS NULL OR b.player_id = $2)
         AND {}
         AND ($4::text IS NULL OR b.status = $4)",
        when_filter("$3")
    );

    let items = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS}, {PLAYER_COLUMNS}
         FROM bookings b JOIN players p ON p.id = b.player_id
         WHERE {filter} {} LIMIT $5 OFFSET $6",
        order_by("$3")
    )))
    .bind(f.pattern)
    .bind(f.player_id)
    .bind(f.when)
    .bind(f.status)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM bookings b JOIN players p ON p.id = b.player_id WHERE {filter}"
    )))
    .bind(f.pattern)
    .bind(f.player_id)
    .bind(f.when)
    .bind(f.status)
    .fetch_one(db)
    .await?;

    Ok((items, total))
}

/// Finds a booking; with `player_id`, only if it belongs to that player.
pub async fn find(
    db: &PgPool,
    id: Uuid,
    player_id: Option<Uuid>,
) -> sqlx::Result<Option<BookingRow>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM bookings b WHERE b.id = $1 AND ($2::uuid IS NULL OR b.player_id = $2)"
    )))
    .bind(id)
    .bind(player_id)
    .fetch_optional(db)
    .await
}

pub async fn find_with_player(db: &PgPool, id: Uuid) -> sqlx::Result<Option<BookingWithPlayer>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS}, {PLAYER_COLUMNS}
         FROM bookings b JOIN players p ON p.id = b.player_id
         WHERE b.id = $1"
    )))
    .bind(id)
    .fetch_optional(db)
    .await
}

/// Fields that can change on Bookeo's side (staff actions) or through an admin update.
pub struct BookingChanges<'a> {
    pub event_id: Option<&'a str>,
    pub num_people: Option<i32>,
    /// `None` keeps the current list.
    pub other_players: Option<&'a [Uuid]>,
    pub status: &'a str,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub price_amount: Option<&'a str>,
    pub price_currency: Option<&'a str>,
}

pub async fn apply_changes(
    db: &PgPool,
    id: Uuid,
    c: &BookingChanges<'_>,
) -> sqlx::Result<BookingRow> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE bookings AS b SET
             event_id = COALESCE($2, b.event_id),
             num_people = COALESCE($3, b.num_people),
             status = $4,
             start_time = $5,
             end_time = $6,
             price_amount = COALESCE($7, b.price_amount),
             price_currency = COALESCE($8, b.price_currency),
             other_players = COALESCE($9, b.other_players),
             updated_at = now()
         WHERE b.id = $1
         RETURNING {COLUMNS}"
    )))
    .bind(id)
    .bind(c.event_id)
    .bind(c.num_people)
    .bind(c.status)
    .bind(c.start_time)
    .bind(c.end_time)
    .bind(c.price_amount)
    .bind(c.price_currency)
    .bind(c.other_players)
    .fetch_one(db)
    .await
}

/// Name and playername for each existing player among `ids` (deleted ones are skipped).
pub async fn player_summaries(
    db: &PgPool,
    ids: &[Uuid],
) -> sqlx::Result<Vec<(Uuid, String, String)>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    sqlx::query_as("SELECT id, full_name, playername FROM players WHERE id = ANY($1)")
        .bind(ids)
        .fetch_all(db)
        .await
}

pub async fn set_status(db: &PgPool, id: Uuid, status: &str) -> sqlx::Result<BookingRow> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE bookings AS b SET status = $2, updated_at = now()
         WHERE b.id = $1
         RETURNING {COLUMNS}"
    )))
    .bind(id)
    .bind(status)
    .fetch_one(db)
    .await
}
