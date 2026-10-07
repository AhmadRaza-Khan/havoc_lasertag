use sqlx::PgPool;
use uuid::Uuid;

/// Saves the token for the player (moving it from whoever had it on that phone before).
pub async fn upsert(db: &PgPool, token: &str, player_id: Uuid, platform: &str) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO push_tokens (token, player_id, platform) VALUES ($1, $2, $3)
         ON CONFLICT (token) DO UPDATE SET player_id = $2, platform = $3, updated_at = now()",
    )
    .bind(token)
    .bind(player_id)
    .bind(platform)
    .execute(db)
    .await?;
    Ok(())
}

/// Removes the token if it belongs to the player.
pub async fn delete(db: &PgPool, token: &str, player_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM push_tokens WHERE token = $1 AND player_id = $2")
        .bind(token)
        .bind(player_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_token(db: &PgPool, token: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM push_tokens WHERE token = $1")
        .bind(token)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_for_player(db: &PgPool, player_id: Uuid) -> sqlx::Result<u64> {
    Ok(sqlx::query("DELETE FROM push_tokens WHERE player_id = $1")
        .bind(player_id)
        .execute(db)
        .await?
        .rows_affected())
}

pub async fn tokens_for(db: &PgPool, player_id: Uuid) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar("SELECT token FROM push_tokens WHERE player_id = $1")
        .bind(player_id)
        .fetch_all(db)
        .await
}
