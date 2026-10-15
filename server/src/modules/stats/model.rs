use chrono::{DateTime, Utc};
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

use super::import::NamedCount;

#[derive(Debug, FromRow)]
pub struct ImportRow {
    pub id: Uuid,
    pub status: String,
    pub source_filename: String,
    pub game_id: Option<Uuid>,
    pub game_started_at: Option<DateTime<Utc>>,
    pub game_ended_at: Option<DateTime<Utc>>,
    pub matched_players: Vec<String>,
    pub unmatched_names: Vec<String>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

/// A player's all-time totals and position (ranked by total points).
#[derive(Debug, FromRow)]
pub struct StandingRow {
    /// `None` until the player has played a game.
    pub position: Option<i64>,
    pub player_id: Uuid,
    pub full_name: String,
    pub playername: String,
    pub avatar_url: Option<String>,
    pub games: i64,
    pub points: i64,
    pub frags: i64,
    pub deactivated: i64,
    pub shots: i64,
    pub hits: i64,
    pub last_played_at: Option<DateTime<Utc>>,
}

/// Everything recorded for one player in one game.
#[derive(Debug, FromRow)]
pub struct GameRecordRow {
    pub game_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub scenario: Option<String>,
    pub number_in_game: i32,
    pub team: Option<String>,
    pub team_score: Option<i32>,
    pub points: i32,
    pub efficiency: f64,
    pub efficiency_kdr: Option<f64>,
    pub frags: i32,
    pub deactivated: i32,
    pub shots: i32,
    pub hits: i32,
    pub accuracy: i32,
    pub captures: Option<i32>,
    pub hits_taken: Option<i32>,
    pub max_combo: Option<i32>,
    pub place_in_team: Option<i32>,
    pub place_in_game: Option<i32>,
    pub nominations: Vec<String>,
    pub victims: Json<Vec<NamedCount>>,
    pub insulters: Json<Vec<NamedCount>>,
}
