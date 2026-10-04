use chrono::{DateTime, Utc};
use sqlx::{PgPool, types::Json};
use uuid::Uuid;

use super::{
    import::ParsedGame,
    model::{GameRecordRow, ImportRow, StandingRow},
};

// ---------------------------------------------------------------------------
// Imports
// ---------------------------------------------------------------------------

const IMPORT_COLUMNS: &str = "i.id, i.status, i.source_filename, i.game_id,
    g.started_at AS game_started_at, g.ended_at AS game_ended_at,
    i.matched_players, i.unmatched_names, i.warnings, i.error, i.created_at, i.finished_at";

pub async fn create_import(
    db: &PgPool,
    filename: &str,
    sha256: &str,
    imported_by: Uuid,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO stat_imports (status, source_filename, file_sha256, imported_by)
         VALUES ('processing', $1, $2, $3) RETURNING id",
    )
    .bind(filename)
    .bind(sha256)
    .bind(imported_by)
    .fetch_one(db)
    .await
}

pub async fn find_import(db: &PgPool, id: Uuid) -> sqlx::Result<Option<ImportRow>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {IMPORT_COLUMNS} FROM stat_imports i LEFT JOIN games g ON g.id = i.game_id
         WHERE i.id = $1"
    )))
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn list_imports(db: &PgPool, limit: i64) -> sqlx::Result<Vec<ImportRow>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {IMPORT_COLUMNS} FROM stat_imports i LEFT JOIN games g ON g.id = i.game_id
         ORDER BY i.created_at DESC LIMIT $1"
    )))
    .bind(limit)
    .fetch_all(db)
    .await
}

pub async fn fail_import(db: &PgPool, id: Uuid, error: &str) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE stat_imports SET status = 'failed', error = $2, finished_at = now()
         WHERE id = $1 AND status = 'processing'",
    )
    .bind(id)
    .bind(error)
    .execute(db)
    .await?;
    Ok(())
}

/// Imports left "processing" by a server restart can never finish: mark them failed.
pub async fn fail_stale_imports(db: &PgPool) -> sqlx::Result<u64> {
    let r = sqlx::query(
        "UPDATE stat_imports
         SET status = 'failed', finished_at = now(),
             error = 'The server restarted while this file was being processed. Please upload it again.'
         WHERE status = 'processing'",
    )
    .execute(db)
    .await?;
    Ok(r.rows_affected())
}

/// When a game with this file or fingerprint was already imported, its start time.
pub async fn existing_game(
    db: &PgPool,
    file_sha256: &str,
    fingerprint: Option<&str>,
) -> sqlx::Result<Option<DateTime<Utc>>> {
    sqlx::query_scalar(
        "SELECT started_at FROM games WHERE file_sha256 = $1 OR fingerprint = $2 LIMIT 1",
    )
    .bind(file_sha256)
    .bind(fingerprint)
    .fetch_optional(db)
    .await
}

/// Lowercased names → existing players, as (id, playername).
pub async fn players_by_playername(
    db: &PgPool,
    playernames: &[String],
) -> sqlx::Result<Vec<(Uuid, String)>> {
    sqlx::query_as("SELECT id, playername FROM players WHERE playername = ANY($1)")
        .bind(playernames)
        .fetch_all(db)
        .await
}

/// Attaches game rows saved before this player existed (name in the file = playername).
/// Returns how many games were linked.
pub async fn link_unclaimed_records(
    db: &PgPool,
    player_id: Uuid,
    playername: &str,
) -> sqlx::Result<u64> {
    let r = sqlx::query(
        "UPDATE game_players SET player_id = $1
         WHERE player_id IS NULL AND LOWER(TRIM(name_in_file)) = $2",
    )
    .bind(player_id)
    .bind(playername)
    .execute(db)
    .await?;
    Ok(r.rows_affected())
}

pub enum SaveOutcome {
    Saved(Uuid),
    /// Lost a race with an identical import (unique fingerprint / file hash).
    Duplicate,
}

/// Saves the game, its player rows and the import result in one transaction: either all of it
/// is stored, or nothing is.
#[allow(clippy::too_many_arguments)]
pub async fn save_game(
    db: &PgPool,
    import_id: Uuid,
    game: &ParsedGame,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    file_sha256: &str,
    source_filename: &str,
    imported_by: Uuid,
    player_ids: &[Option<Uuid>],
    matched: &[String],
    unmatched: &[String],
) -> sqlx::Result<SaveOutcome> {
    let mut tx = db.begin().await?;

    let game_id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO games (started_at, ended_at, scenario, fingerprint, file_sha256,
                            source_filename, imported_by, warnings)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT DO NOTHING
         RETURNING id",
    )
    .bind(started_at)
    .bind(ended_at)
    .bind(&game.scenario)
    .bind(&game.fingerprint)
    .bind(file_sha256)
    .bind(source_filename)
    .bind(imported_by)
    .bind(&game.warnings)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(game_id) = game_id else {
        return Ok(SaveOutcome::Duplicate);
    };

    for (p, player_id) in game.players.iter().zip(player_ids) {
        let r = &p.row;
        let ps = p.personal.clone().unwrap_or_default();
        let has_page = p.personal.is_some();
        sqlx::query(
            "INSERT INTO game_players (
                 game_id, player_id, name_in_file, number_in_game,
                 points, efficiency, frags, deactivated, shots, hits, accuracy,
                 personal_page, team, team_score, efficiency_kdr, captures, hits_taken, max_combo,
                 place_in_team, place_in_game, nominations, victims, insulters)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                     $18, $19, $20, $21, $22, $23)",
        )
        .bind(game_id)
        .bind(player_id)
        .bind(&r.name)
        .bind(r.number)
        .bind(r.points)
        .bind(r.efficiency)
        .bind(r.frags)
        .bind(r.deactivated)
        .bind(r.shots)
        .bind(r.hits)
        .bind(r.accuracy)
        .bind(has_page.then_some(ps.page as i32))
        .bind(ps.team)
        .bind(ps.team_score)
        .bind(ps.efficiency_kdr)
        .bind(ps.captures)
        .bind(ps.hits_taken)
        .bind(ps.max_combo)
        .bind(ps.place_in_team)
        .bind(ps.place_in_game)
        .bind(ps.nominations)
        .bind(Json(ps.victims))
        .bind(Json(ps.insulters))
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "UPDATE stat_imports
         SET status = 'completed', game_id = $2, matched_players = $3, unmatched_names = $4,
             warnings = $5, finished_at = now()
         WHERE id = $1",
    )
    .bind(import_id)
    .bind(game_id)
    .bind(matched)
    .bind(unmatched)
    .bind(&game.warnings)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(SaveOutcome::Saved(game_id))
}

// ---------------------------------------------------------------------------
// Standings and records
// ---------------------------------------------------------------------------

/// Every player with all-time totals. Standard competition ranking among players who have
/// played (tied points share a position: 1, 2, 2, 4); players without games have no position.
const STANDINGS: &str = "
    SELECT CASE WHEN COUNT(gp.id) > 0 THEN
               RANK() OVER (PARTITION BY COUNT(gp.id) > 0
                            ORDER BY COALESCE(SUM(gp.points), 0) DESC)
           END AS position,
           p.id AS player_id, p.full_name, p.playername,
           COUNT(gp.id) AS games,
           COALESCE(SUM(gp.points), 0)::BIGINT AS points,
           COALESCE(SUM(gp.frags), 0)::BIGINT AS frags,
           COALESCE(SUM(gp.deactivated), 0)::BIGINT AS deactivated,
           COALESCE(SUM(gp.shots), 0)::BIGINT AS shots,
           COALESCE(SUM(gp.hits), 0)::BIGINT AS hits,
           MAX(g.started_at) AS last_played_at
    FROM players p
    LEFT JOIN game_players gp ON gp.player_id = p.id
    LEFT JOIN games g ON g.id = gp.game_id
    GROUP BY p.id";

pub async fn leaderboard(db: &PgPool) -> sqlx::Result<Vec<StandingRow>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{STANDINGS} ORDER BY COUNT(gp.id) > 0 DESC, points DESC, LOWER(p.full_name), p.playername"
    )))
    .fetch_all(db)
    .await
}

pub async fn player_count(db: &PgPool) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM players")
        .fetch_one(db)
        .await
}

pub async fn standing(db: &PgPool, player_id: Uuid) -> sqlx::Result<Option<StandingRow>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT * FROM ({STANDINGS}) s WHERE s.player_id = $1"
    )))
    .bind(player_id)
    .fetch_optional(db)
    .await
}

const RECORD_COLUMNS: &str = "g.id AS game_id, g.started_at, g.ended_at, g.scenario,
    gp.number_in_game, gp.team, gp.team_score, gp.points, gp.efficiency, gp.efficiency_kdr,
    gp.frags, gp.deactivated, gp.shots, gp.hits, gp.accuracy, gp.captures, gp.hits_taken,
    gp.max_combo, gp.place_in_team, gp.place_in_game, gp.nominations, gp.victims, gp.insulters";

/// The player's own game records, newest first.
pub async fn player_records(
    db: &PgPool,
    player_id: Uuid,
    limit: i64,
    offset: i64,
) -> sqlx::Result<(Vec<GameRecordRow>, i64)> {
    let items = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {RECORD_COLUMNS} FROM game_players gp JOIN games g ON g.id = gp.game_id
         WHERE gp.player_id = $1
         ORDER BY g.started_at DESC, g.id
         LIMIT $2 OFFSET $3"
    )))
    .bind(player_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;
    let total = sqlx::query_scalar("SELECT COUNT(*) FROM game_players WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(db)
        .await?;
    Ok((items, total))
}

/// One of the player's records; `None` if they didn't play that game.
pub async fn player_record(
    db: &PgPool,
    player_id: Uuid,
    game_id: Uuid,
) -> sqlx::Result<Option<GameRecordRow>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {RECORD_COLUMNS} FROM game_players gp JOIN games g ON g.id = gp.game_id
         WHERE gp.player_id = $1 AND gp.game_id = $2"
    )))
    .bind(player_id)
    .bind(game_id)
    .fetch_optional(db)
    .await
}
