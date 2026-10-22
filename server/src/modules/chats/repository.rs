use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::model::{ChatMessage, ChatPlayer, ConversationRow};

/// Selects a message with its recipient (the conversation member who isn't the sender).
const MESSAGE_COLUMNS: &str = "m.id, m.conversation_id, m.sender_id,
    CASE WHEN c.player_a = m.sender_id THEN c.player_b ELSE c.player_a END AS recipient_id,
    m.body, m.created_at, m.read_at";

/// Conversations are stored with the pair ordered (player_a < player_b).
fn ordered(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a < b { (a, b) } else { (b, a) }
}

/// Every player except `me`, by name. `pattern` is an escaped ILIKE pattern.
pub async fn list_players(
    db: &PgPool,
    me: Uuid,
    pattern: Option<&str>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<(Vec<ChatPlayer>, i64)> {
    const FILTER: &str = "id <> $1 AND ($2::text IS NULL OR playername ILIKE $2 OR full_name ILIKE $2)
        AND NOT EXISTS (SELECT 1 FROM player_blocks b WHERE b.blocker_id = $1 AND b.blocked_id = players.id)";

    let items = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT id, full_name, playername, last_seen_at,
                CASE WHEN avatar_updated_at IS NULL THEN NULL ELSE '/api/avatars/' || id || '?v=' || floor(extract(epoch FROM avatar_updated_at) * 1000)::bigint END AS avatar_url FROM players
         WHERE {FILTER} ORDER BY full_name, playername LIMIT $3 OFFSET $4"
    )))
    .bind(me)
    .bind(pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM players WHERE {FILTER}"
    )))
    .bind(me)
    .bind(pattern)
    .fetch_one(db)
    .await?;

    Ok((items, total))
}

pub async fn find_player(db: &PgPool, id: Uuid) -> sqlx::Result<Option<ChatPlayer>> {
    sqlx::query_as("SELECT id, full_name, playername, last_seen_at,
                CASE WHEN avatar_updated_at IS NULL THEN NULL ELSE '/api/avatars/' || id || '?v=' || floor(extract(epoch FROM avatar_updated_at) * 1000)::bigint END AS avatar_url FROM players WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

/// `me`'s conversations with the last message and unread count, most recent first.
pub async fn list_conversations(db: &PgPool, me: Uuid) -> sqlx::Result<Vec<ConversationRow>> {
    sqlx::query_as(
        "SELECT c.id,
                o.id AS other_id, o.full_name AS other_full_name,
                o.playername AS other_playername, o.last_seen_at AS other_last_seen_at,
                CASE WHEN o.avatar_updated_at IS NULL THEN NULL ELSE '/api/avatars/' || o.id || '?v=' || floor(extract(epoch FROM o.avatar_updated_at) * 1000)::bigint END AS other_avatar_url,
                lm.id AS last_message_id, lm.sender_id AS last_message_sender_id,
                lm.body AS last_message_body, lm.created_at AS last_message_created_at,
                lm.read_at AS last_message_read_at,
                (SELECT COUNT(*) FROM chat_messages u
                  WHERE u.conversation_id = c.id AND u.sender_id <> $1 AND u.read_at IS NULL
                ) AS unread_count
         FROM chat_conversations c
         JOIN players o ON o.id = CASE WHEN c.player_a = $1 THEN c.player_b ELSE c.player_a END
         LEFT JOIN LATERAL (
             SELECT id, sender_id, body, created_at, read_at FROM chat_messages
             WHERE conversation_id = c.id
             ORDER BY created_at DESC, id DESC LIMIT 1
         ) lm ON true
         WHERE (c.player_a = $1 OR c.player_b = $1)
           AND NOT EXISTS (SELECT 1 FROM player_blocks b WHERE b.blocker_id = $1 AND b.blocked_id = o.id)
         ORDER BY c.last_message_at DESC",
    )
    .bind(me)
    .fetch_all(db)
    .await
}

/// Messages between two players, newest first. With `before`, only messages older than that
/// message (cursor pagination). Returns at most `limit`.
pub async fn list_messages(
    db: &PgPool,
    me: Uuid,
    other: Uuid,
    before: Option<Uuid>,
    limit: i64,
) -> sqlx::Result<Vec<ChatMessage>> {
    let (a, b) = ordered(me, other);
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {MESSAGE_COLUMNS}
         FROM chat_messages m JOIN chat_conversations c ON c.id = m.conversation_id
         WHERE c.player_a = $1 AND c.player_b = $2
           AND ($3::uuid IS NULL OR (m.created_at, m.id) <
                (SELECT created_at, id FROM chat_messages WHERE id = $3 AND conversation_id = c.id))
         ORDER BY m.created_at DESC, m.id DESC
         LIMIT $4"
    )))
    .bind(a)
    .bind(b)
    .bind(before)
    .bind(limit)
    .fetch_all(db)
    .await
}

/// Stores a message, creating the conversation on the first one.
pub async fn insert_message(
    db: &PgPool,
    sender: Uuid,
    recipient: Uuid,
    body: &str,
) -> sqlx::Result<ChatMessage> {
    let (a, b) = ordered(sender, recipient);
    let mut tx = db.begin().await?;

    let conversation_id: Uuid = sqlx::query_scalar(
        "INSERT INTO chat_conversations (player_a, player_b) VALUES ($1, $2)
         ON CONFLICT (player_a, player_b) DO UPDATE SET last_message_at = now()
         RETURNING id",
    )
    .bind(a)
    .bind(b)
    .fetch_one(&mut *tx)
    .await?;

    let message = sqlx::query_as(
        "INSERT INTO chat_messages (conversation_id, sender_id, body) VALUES ($1, $2, $3)
         RETURNING id, conversation_id, sender_id, $4::uuid AS recipient_id, body, created_at, read_at",
    )
    .bind(conversation_id)
    .bind(sender)
    .bind(body)
    .bind(recipient)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(message)
}

/// Marks every unread message `other` sent to `me` as read. Returns the read time if any
/// message changed.
pub async fn mark_read(db: &PgPool, me: Uuid, other: Uuid) -> sqlx::Result<Option<DateTime<Utc>>> {
    let (a, b) = ordered(me, other);
    let updated: Vec<DateTime<Utc>> = sqlx::query_scalar(
        "UPDATE chat_messages m SET read_at = now()
         FROM chat_conversations c
         WHERE c.id = m.conversation_id AND c.player_a = $1 AND c.player_b = $2
           AND m.sender_id = $3 AND m.read_at IS NULL
         RETURNING m.read_at",
    )
    .bind(a)
    .bind(b)
    .bind(other)
    .fetch_all(db)
    .await?;
    Ok(updated.into_iter().next())
}

/// Records that the player is/was connected now. `None` if the player no longer exists.
pub async fn touch_last_seen(db: &PgPool, player: Uuid) -> sqlx::Result<Option<DateTime<Utc>>> {
    sqlx::query_scalar(
        "UPDATE players SET last_seen_at = now() WHERE id = $1 RETURNING last_seen_at",
    )
    .bind(player)
    .fetch_optional(db)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_pairs() {
        let (x, y) = (Uuid::from_u128(1), Uuid::from_u128(2));
        assert_eq!(ordered(x, y), (x, y));
        assert_eq!(ordered(y, x), (x, y));
    }
}
