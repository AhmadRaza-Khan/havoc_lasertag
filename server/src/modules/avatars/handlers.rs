use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, State},
    http::{HeaderValue, Response, header},
};
use uuid::Uuid;

use super::MAX_BYTES;
use crate::{
    error::{AppError, AppResult},
    modules::{
        player_auth::AuthPlayer,
        players::{Player, repository as players},
    },
    state::AppState,
};

/// `multipart/form-data` with the image in a field named `file`. Returns the updated player.
pub async fn upload(
    State(state): State<AppState>,
    player: AuthPlayer,
    mut multipart: Multipart,
) -> AppResult<Json<Player>> {
    let mut image = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid upload: {e}")))?
    {
        if field.name() == Some("file") {
            let bytes = field.bytes().await.map_err(|_| {
                AppError::BadRequest("The picture is too large (max 900 KB)".into())
            })?;
            image = Some(bytes);
            break;
        }
    }
    let bytes = image.ok_or_else(|| AppError::BadRequest("No picture in the upload".into()))?;
    if bytes.len() > MAX_BYTES {
        return Err(AppError::BadRequest(
            "The picture is too large (max 900 KB)".into(),
        ));
    }
    // Trust the bytes, not the client's file name or content type.
    let content_type = sniff(&bytes)
        .ok_or_else(|| AppError::BadRequest("Use a JPEG, PNG or WebP picture".into()))?;

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO player_avatars (player_id, content_type, data) VALUES ($1, $2, $3)
         ON CONFLICT (player_id) DO UPDATE SET content_type = $2, data = $3, updated_at = now()",
    )
    .bind(player.id)
    .bind(content_type)
    .bind(bytes.as_ref())
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE players SET avatar_updated_at = now() WHERE id = $1")
        .bind(player.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    tracing::info!(player_id = %player.id, bytes = bytes.len(), "profile picture updated");
    Ok(Json(me(&state, player.id).await?))
}

/// Removes the picture. Returns the updated player.
pub async fn remove(State(state): State<AppState>, player: AuthPlayer) -> AppResult<Json<Player>> {
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM player_avatars WHERE player_id = $1")
        .bind(player.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE players SET avatar_updated_at = NULL WHERE id = $1")
        .bind(player.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(me(&state, player.id).await?))
}

/// The image. The URL carries a version (`?v=`), so it's cached for a year: a new upload gets a
/// new URL.
pub async fn serve(
    State(state): State<AppState>,
    Path(player_id): Path<Uuid>,
) -> AppResult<Response<Body>> {
    let row: Option<(String, Vec<u8>)> =
        sqlx::query_as("SELECT content_type, data FROM player_avatars WHERE player_id = $1")
            .bind(player_id)
            .fetch_optional(&state.db)
            .await?;
    let (content_type, data) = row.ok_or_else(|| AppError::NotFound("No picture".into()))?;
    let mut res = Response::new(Body::from(data));
    let headers = res.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&content_type).unwrap_or(HeaderValue::from_static("image/jpeg")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    Ok(res)
}

async fn me(state: &AppState, id: Uuid) -> AppResult<Player> {
    players::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Player no longer exists".into()))
}

/// The image type from its first bytes (magic numbers), or `None` if it isn't one we accept.
fn sniff(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, ..] => Some("image/png"),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => Some("image/webp"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_image_types() {
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]), Some("image/jpeg"));
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n...."), Some("image/png"));
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff(b"<svg xmlns=..."), None);
        assert_eq!(sniff(b"GIF89a"), None);
        assert_eq!(sniff(&[]), None);
    }
}
