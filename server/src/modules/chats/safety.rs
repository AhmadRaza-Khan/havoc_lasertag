//! Blocking and reporting (App Store guideline 1.2, Google Play UGC policy).
//!
//! A block stops messages both ways and hides the blocked player from the blocker's chat list
//! and directory. Reports keep a copy of the reported message; staff review them in the admin
//! panel. Havoc Admin (staff) can't be blocked, so players can always reach support.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::model::ChatPlayer;
use crate::{
    error::{AppError, AppResult},
    modules::{auth::AuthUser, player_auth::AuthPlayer, players::HAVOC_ADMIN_ID},
    state::AppState,
};

const MAX_DETAILS: usize = 1000;
const REASONS: &[&str] = &["spam", "harassment", "inappropriate", "other"];

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Whether either player has blocked the other.
pub async fn blocked_between(db: &PgPool, a: Uuid, b: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM player_blocks
                        WHERE (blocker_id = $1 AND blocked_id = $2) OR (blocker_id = $2 AND blocked_id = $1))",
    )
    .bind(a)
    .bind(b)
    .fetch_one(db)
    .await
}

// ---------------------------------------------------------------------------
// Mobile app
// ---------------------------------------------------------------------------

/// `POST /api/player/chats/players/{id}/block`
pub async fn block(
    State(state): State<AppState>,
    me: AuthPlayer,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    if id == me.id {
        return Err(AppError::BadRequest("You can't block yourself".into()));
    }
    if id == HAVOC_ADMIN_ID {
        return Err(AppError::BadRequest(
            "Havoc Admin can't be blocked. Report a message instead.".into(),
        ));
    }
    let inserted = sqlx::query(
        "INSERT INTO player_blocks (blocker_id, blocked_id)
         SELECT $1, id FROM players WHERE id = $2
         ON CONFLICT DO NOTHING",
    )
    .bind(me.id)
    .bind(id)
    .execute(&state.db)
    .await?;
    if inserted.rows_affected() == 0
        && !sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM players WHERE id = $1)")
            .bind(id)
            .fetch_one(&state.db)
            .await?
    {
        return Err(AppError::NotFound("Player not found".into()));
    }
    tracing::info!(blocker = %me.id, blocked = %id, "player blocked");
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/player/chats/players/{id}/block`
pub async fn unblock(
    State(state): State<AppState>,
    me: AuthPlayer,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    sqlx::query("DELETE FROM player_blocks WHERE blocker_id = $1 AND blocked_id = $2")
        .bind(me.id)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/player/chats/blocks`: the players I blocked, most recent first.
pub async fn blocks(
    State(state): State<AppState>,
    me: AuthPlayer,
) -> AppResult<Json<Vec<ChatPlayer>>> {
    let players: Vec<ChatPlayer> = sqlx::query_as(
        "SELECT p.id, p.full_name, p.playername, p.last_seen_at,
                CASE WHEN p.avatar_updated_at IS NULL THEN NULL ELSE '/api/avatars/' || p.id || '?v=' || floor(extract(epoch FROM p.avatar_updated_at) * 1000)::bigint END AS avatar_url
         FROM player_blocks b JOIN players p ON p.id = b.blocked_id
         WHERE b.blocker_id = $1 ORDER BY b.created_at DESC",
    )
    .bind(me.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(players))
}

#[derive(Deserialize)]
pub struct ReportRequest {
    pub player_id: Uuid,
    /// The reported message, if reporting one.
    pub message_id: Option<Uuid>,
    /// `spam`, `harassment`, `inappropriate` or `other`.
    pub reason: String,
    pub details: Option<String>,
}

/// `POST /api/player/chats/reports`: report a player, or one of their messages to me.
pub async fn report(
    State(state): State<AppState>,
    me: AuthPlayer,
    Json(req): Json<ReportRequest>,
) -> AppResult<StatusCode> {
    if !REASONS.contains(&req.reason.as_str()) {
        return Err(AppError::BadRequest("Choose a reason".into()));
    }
    if req.player_id == me.id {
        return Err(AppError::BadRequest("You can't report yourself".into()));
    }
    let details = req
        .details
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty());
    if details
        .as_ref()
        .is_some_and(|d| d.chars().count() > MAX_DETAILS)
    {
        return Err(AppError::BadRequest(format!(
            "Details can be at most {MAX_DETAILS} characters"
        )));
    }
    let names: Option<(String, String)> = sqlx::query_as(
        "SELECT r.playername, p.playername FROM players r, players p WHERE r.id = $1 AND p.id = $2",
    )
    .bind(me.id)
    .bind(req.player_id)
    .fetch_optional(&state.db)
    .await?;
    let (reporter_name, reported_name) =
        names.ok_or_else(|| AppError::NotFound("Player not found".into()))?;

    // Only a message the reported player sent to me can be attached.
    let message_body: Option<String> = match req.message_id {
        Some(mid) => Some(
            sqlx::query_scalar(
                "SELECT m.body FROM chat_messages m JOIN chat_conversations c ON c.id = m.conversation_id
                 WHERE m.id = $1 AND m.sender_id = $2 AND $3 IN (c.player_a, c.player_b)",
            )
            .bind(mid)
            .bind(req.player_id)
            .bind(me.id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::NotFound("Message not found".into()))?,
        ),
        None => None,
    };

    sqlx::query(
        "INSERT INTO chat_reports (reporter_id, reported_id, reporter_playername, reported_playername,
                                   message_id, message_body, reason, details)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(me.id)
    .bind(req.player_id)
    .bind(reporter_name)
    .bind(reported_name)
    .bind(req.message_id)
    .bind(message_body)
    .bind(&req.reason)
    .bind(details)
    .execute(&state.db)
    .await?;
    tracing::info!(reporter = %me.id, reported = %req.player_id, reason = %req.reason, "chat report");
    Ok(StatusCode::CREATED)
}

// ---------------------------------------------------------------------------
// Admin panel
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, FromRow)]
pub struct Report {
    pub id: Uuid,
    pub reporter_id: Option<Uuid>,
    pub reported_id: Option<Uuid>,
    pub reporter_playername: String,
    pub reported_playername: String,
    pub message_id: Option<Uuid>,
    pub message_body: Option<String>,
    pub reason: String,
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct ReportsQuery {
    /// `open` (default) or `resolved`.
    pub status: Option<String>,
}

/// `GET /api/chats/reports?status=open|resolved`: newest first (at most 200).
pub async fn admin_reports(
    State(state): State<AppState>,
    _admin: AuthUser,
    Query(q): Query<ReportsQuery>,
) -> AppResult<Json<Vec<Report>>> {
    let resolved = q.status.as_deref() == Some("resolved");
    let reports = sqlx::query_as(
        "SELECT id, reporter_id, reported_id, reporter_playername, reported_playername, message_id,
                message_body, reason, details, created_at, resolved_at
         FROM chat_reports WHERE (resolved_at IS NOT NULL) = $1
         ORDER BY created_at DESC LIMIT 200",
    )
    .bind(resolved)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(reports))
}

/// `POST /api/chats/reports/{id}/resolve`: staff handled it.
pub async fn admin_resolve(
    State(state): State<AppState>,
    admin: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let done = sqlx::query(
        "UPDATE chat_reports SET resolved_at = now() WHERE id = $1 AND resolved_at IS NULL",
    )
    .bind(id)
    .execute(&state.db)
    .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Report not found or already resolved".into(),
        ));
    }
    tracing::info!(report = %id, admin = %admin.id, "chat report resolved");
    Ok(StatusCode::NO_CONTENT)
}
