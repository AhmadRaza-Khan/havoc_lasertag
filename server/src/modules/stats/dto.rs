use chrono::{DateTime, FixedOffset, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    import::NamedCount,
    model::{GameRecordRow, ImportRow, StandingRow},
};

#[derive(Debug, Serialize)]
pub struct ImportResponse {
    pub id: Uuid,
    /// `processing`, `completed` or `failed`.
    pub status: String,
    pub source_filename: String,
    pub game: Option<GameRef>,
    /// Playernames whose stats were saved.
    pub matched_players: Vec<String>,
    /// Names in the file with no matching player (their rows are kept, unlinked).
    pub unmatched_names: Vec<String>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct GameRef {
    pub id: Uuid,
    pub started_at: DateTime<FixedOffset>,
    pub ended_at: DateTime<FixedOffset>,
}

impl ImportResponse {
    pub fn from_row(r: ImportRow, tz: Tz) -> Self {
        let game = match (r.game_id, r.game_started_at, r.game_ended_at) {
            (Some(id), Some(s), Some(e)) => Some(GameRef {
                id,
                started_at: local(s, tz),
                ended_at: local(e, tz),
            }),
            _ => None,
        };
        Self {
            id: r.id,
            status: r.status,
            source_filename: r.source_filename,
            game,
            matched_players: r.matched_players,
            unmatched_names: r.unmatched_names,
            warnings: r.warnings,
            error: r.error,
            created_at: r.created_at,
            finished_at: r.finished_at,
        }
    }
}

/// A player's all-time totals: the general report (same columns as the PDF's summary page).
#[derive(Debug, Serialize)]
pub struct Standing {
    /// 1 = most points. Tied points share a position. `null` until the player has played.
    pub position: Option<i64>,
    pub player: PlayerRef,
    pub games: i64,
    pub points: i64,
    /// Frags per deactivation (frags if never deactivated) — the PDF's "Efficiency".
    pub efficiency: f64,
    pub frags: i64,
    pub deactivated: i64,
    pub shots: i64,
    pub hits: i64,
    /// Hits per shot, in percent.
    pub accuracy: i64,
    pub last_played_at: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Serialize)]
pub struct PlayerRef {
    pub id: Uuid,
    pub full_name: String,
    pub playername: String,
}

impl Standing {
    pub fn from_row(r: StandingRow, tz: Tz) -> Self {
        Self {
            position: r.position,
            player: PlayerRef {
                id: r.player_id,
                full_name: r.full_name,
                playername: r.playername,
            },
            games: r.games,
            points: r.points,
            efficiency: efficiency(r.frags, r.deactivated),
            frags: r.frags,
            deactivated: r.deactivated,
            shots: r.shots,
            hits: r.hits,
            accuracy: accuracy(r.hits, r.shots),
            last_played_at: r.last_played_at.map(|t| local(t, tz)),
        }
    }
}

/// One row of the leaderboard players see: other players appear by playername only.
#[derive(Debug, Clone, Serialize)]
pub struct PublicStanding {
    /// 1 = most points. Tied points share a position.
    pub position: i64,
    pub player: PublicPlayer,
    pub games: i64,
    pub points: i64,
    /// Frags per deactivation (frags if never deactivated).
    pub efficiency: f64,
    pub frags: i64,
    pub deactivated: i64,
    /// Hits per shot, in percent.
    pub accuracy: i64,
    pub last_played_at: Option<DateTime<FixedOffset>>,
    /// The signed-in player's own row.
    pub is_me: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicPlayer {
    pub id: Uuid,
    pub playername: String,
}

impl PublicStanding {
    /// `None` for players who haven't played yet (they aren't ranked).
    pub fn from_row(r: StandingRow, tz: Tz, me: Uuid) -> Option<Self> {
        Some(Self {
            position: r.position?,
            is_me: r.player_id == me,
            player: PublicPlayer {
                id: r.player_id,
                playername: r.playername,
            },
            games: r.games,
            points: r.points,
            efficiency: efficiency(r.frags, r.deactivated),
            frags: r.frags,
            deactivated: r.deactivated,
            accuracy: accuracy(r.hits, r.shots),
            last_played_at: r.last_played_at.map(|t| local(t, tz)),
        })
    }
}

/// The leaderboard as players see it (mobile app).
#[derive(Debug, Serialize)]
pub struct Leaderboard {
    /// The top ranked players, best first.
    pub standings: Vec<PublicStanding>,
    /// The signed-in player's row, even when it's below the top. `null` until they've played.
    pub me: Option<PublicStanding>,
    /// Everyone who has played at least one game.
    pub total_ranked: i64,
}

/// The player's own standing (mobile app).
#[derive(Debug, Serialize)]
pub struct MyStats {
    #[serde(flatten)]
    pub standing: Standing,
    pub total_players: i64,
}

/// Everything recorded for one player in one game.
#[derive(Debug, Serialize)]
pub struct GameRecord {
    pub game: GameInfo,
    /// The player's ID in that game.
    pub number_in_game: i32,
    pub team: Option<String>,
    pub team_score: Option<i32>,
    pub points: i32,
    /// From the summary page (frags per deactivation).
    pub efficiency: f64,
    /// From the player's own page ("Efficiency KDR").
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
    pub victims: Vec<NamedCount>,
    pub insulters: Vec<NamedCount>,
}

#[derive(Debug, Serialize)]
pub struct GameInfo {
    pub id: Uuid,
    pub started_at: DateTime<FixedOffset>,
    pub ended_at: DateTime<FixedOffset>,
    pub scenario: Option<String>,
}

impl GameRecord {
    pub fn from_row(r: GameRecordRow, tz: Tz) -> Self {
        Self {
            game: GameInfo {
                id: r.game_id,
                started_at: local(r.started_at, tz),
                ended_at: local(r.ended_at, tz),
                scenario: r.scenario,
            },
            number_in_game: r.number_in_game,
            team: r.team,
            team_score: r.team_score,
            points: r.points,
            efficiency: r.efficiency,
            efficiency_kdr: r.efficiency_kdr,
            frags: r.frags,
            deactivated: r.deactivated,
            shots: r.shots,
            hits: r.hits,
            accuracy: r.accuracy,
            captures: r.captures,
            hits_taken: r.hits_taken,
            max_combo: r.max_combo,
            place_in_team: r.place_in_team,
            place_in_game: r.place_in_game,
            nominations: r.nominations,
            victims: r.victims.0,
            insulters: r.insulters.0,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

fn local(t: DateTime<Utc>, tz: Tz) -> DateTime<FixedOffset> {
    t.with_timezone(&tz).fixed_offset()
}

/// Same definition the arena's summary page uses (verified on real reports): frags divided by
/// deactivations, or the frags themselves if never deactivated. Two decimals.
pub fn efficiency(frags: i64, deactivated: i64) -> f64 {
    let raw = if deactivated > 0 {
        frags as f64 / deactivated as f64
    } else {
        frags as f64
    };
    (raw * 100.0).round() / 100.0
}

/// Hits per shot in percent, rounded (0 when no shots) — matches the summary page.
pub fn accuracy(hits: i64, shots: i64) -> i64 {
    if shots <= 0 {
        0
    } else {
        ((hits as f64 * 100.0) / shots as f64).round() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_match_the_pdf_summary_formulas() {
        // (frags, deactivated, expected efficiency) from the real report
        assert_eq!(efficiency(2, 1), 2.0); // Elios, Wolf
        assert_eq!(efficiency(0, 1), 0.0); // Zef, Iridi
        assert_eq!(efficiency(0, 0), 0.0); // Aaron
        assert_eq!(efficiency(3, 0), 3.0);
        assert_eq!(efficiency(2, 3), 0.67);
        // (hits, shots, expected accuracy)
        assert_eq!(accuracy(4, 9), 44); // Elios
        assert_eq!(accuracy(1, 2), 50); // Zef
        assert_eq!(accuracy(6, 15), 40); // Wolf
        assert_eq!(accuracy(0, 0), 0); // Aaron
    }

    fn row(position: Option<i64>, id: u128) -> StandingRow {
        StandingRow {
            position,
            player_id: Uuid::from_u128(id),
            full_name: "Real Name".into(),
            playername: "wolf".into(),
            games: 3,
            points: 120,
            frags: 6,
            deactivated: 3,
            shots: 20,
            hits: 5,
            last_played_at: None,
        }
    }

    #[test]
    fn public_standing_hides_private_details() {
        let me = Uuid::from_u128(7);
        let tz: Tz = "Asia/Qatar".parse().unwrap();
        assert!(
            PublicStanding::from_row(row(None, 1), tz, me).is_none(),
            "not played yet: not ranked"
        );

        let other = PublicStanding::from_row(row(Some(2), 1), tz, me).unwrap();
        assert!(!other.is_me);
        assert_eq!((other.efficiency, other.accuracy), (2.0, 25));
        let json = serde_json::to_string(&other).unwrap();
        assert!(json.contains("\"playername\":\"wolf\""));
        assert!(
            !json.contains("Real Name"),
            "full names stay private: {json}"
        );

        assert!(
            PublicStanding::from_row(row(Some(1), 7), tz, me)
                .unwrap()
                .is_me
        );
    }
}
