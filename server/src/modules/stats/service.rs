use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use sha2::{Digest, Sha256};
use tokio::sync::Semaphore;
use uuid::Uuid;

use super::{
    import::{self, ImportError},
    model::{GameRecordRow, ImportRow, StandingRow},
    repository::{self, SaveOutcome},
};
use crate::{
    error::{AppError, AppResult},
    shared::{
        ocr::Ocr,
        pagination::{PageParams, Paginated},
    },
    state::AppState,
};

/// Real reports are ~5 MB.
pub const MAX_UPLOAD_BYTES: usize = 25 * 1024 * 1024;

/// OCR is CPU-heavy: one import at a time, others queue.
static IMPORT_SLOT: Semaphore = Semaphore::const_new(1);

fn upload_dir() -> PathBuf {
    std::env::temp_dir().join("havoc-stat-imports")
}

/// On startup: imports interrupted by a restart are marked failed and their temp files removed.
pub async fn recover(state: &AppState) -> AppResult<()> {
    let stale = repository::fail_stale_imports(&state.db).await?;
    if stale > 0 {
        tracing::warn!(stale, "marked interrupted stat imports as failed");
    }
    if let Ok(mut dir) = tokio::fs::read_dir(upload_dir()).await {
        while let Ok(Some(entry)) = dir.next_entry().await {
            let _ = tokio::fs::remove_file(entry.path()).await;
        }
    }
    Ok(())
}

/// Validates and stores the upload in a temp file, then processes it in the background.
/// Returns at once with the import in `processing` state; poll it for the outcome.
pub async fn start_import(
    state: &AppState,
    admin_id: Uuid,
    filename: &str,
    bytes: Vec<u8>,
) -> AppResult<ImportRow> {
    let filename = sanitize_filename(filename);
    if !filename.to_lowercase().ends_with(".pdf") {
        return Err(AppError::BadRequest("Please upload a .pdf file".into()));
    }
    if bytes.is_empty() {
        return Err(AppError::BadRequest("The file is empty".into()));
    }
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err(AppError::BadRequest(format!(
            "The file is too large (max {} MB)",
            MAX_UPLOAD_BYTES / 1024 / 1024
        )));
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err(AppError::BadRequest("The file is not a PDF".into()));
    }

    let sha256 = hex::encode(Sha256::digest(&bytes));
    if let Some(played) = repository::existing_game(&state.db, &sha256, None).await? {
        return Err(already_imported(played, state.config.bookeo.timezone));
    }

    let import_id = repository::create_import(&state.db, &filename, &sha256, admin_id).await?;

    let dir = upload_dir();
    let path = dir.join(format!("{import_id}.pdf"));
    if let Err(e) = write_private(&dir, &path, &bytes).await {
        let _ = repository::fail_import(&state.db, import_id, "Could not store the upload").await;
        return Err(AppError::Internal(format!("storing upload: {e}")));
    }

    let job = ImportJob {
        state: state.clone(),
        import_id,
        file: TempFile(path),
        sha256,
        filename,
        admin_id,
    };
    tokio::spawn(job.run());

    repository::find_import(&state.db, import_id)
        .await?
        .ok_or_else(|| AppError::Internal("import vanished".into()))
}

/// A newly created player gets the stats of earlier games that listed their name.
pub async fn link_unclaimed_records(state: &AppState, player_id: Uuid, playername: &str) {
    match repository::link_unclaimed_records(&state.db, player_id, playername).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(%player_id, games = n, "linked earlier game stats to new player"),
        Err(e) => tracing::error!(%player_id, "linking earlier game stats failed: {e}"),
    }
}

pub async fn get_import(state: &AppState, id: Uuid) -> AppResult<ImportRow> {
    repository::find_import(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Import not found".into()))
}

pub async fn recent_imports(state: &AppState) -> AppResult<Vec<ImportRow>> {
    Ok(repository::list_imports(&state.db, 10).await?)
}

pub async fn leaderboard(state: &AppState) -> AppResult<Vec<StandingRow>> {
    Ok(repository::leaderboard(&state.db).await?)
}

pub async fn player_standing(state: &AppState, player_id: Uuid) -> AppResult<StandingRow> {
    repository::standing(&state.db, player_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Player no longer exists".into()))
}

pub async fn player_records(
    state: &AppState,
    player_id: Uuid,
    params: PageParams,
) -> AppResult<Paginated<GameRecordRow>> {
    let (items, total) =
        repository::player_records(&state.db, player_id, params.per_page(), params.offset())
            .await?;
    Ok(Paginated::new(items, total, &params))
}

pub async fn player_record(
    state: &AppState,
    player_id: Uuid,
    game_id: Uuid,
) -> AppResult<GameRecordRow> {
    repository::player_record(&state.db, player_id, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game record not found".into()))
}

// ---------------------------------------------------------------------------
// Background processing
// ---------------------------------------------------------------------------

struct ImportJob {
    state: AppState,
    import_id: Uuid,
    file: TempFile,
    sha256: String,
    filename: String,
    admin_id: Uuid,
}

impl ImportJob {
    async fn run(self) {
        let import_id = self.import_id;
        let db = self.state.db.clone();
        // `self.file` is dropped (and the temp file deleted) when this returns, whatever happens.
        match self.process().await {
            Ok(game_id) => tracing::info!(%import_id, %game_id, "stat import completed"),
            Err(message) => {
                tracing::warn!(%import_id, "stat import failed: {message}");
                if let Err(e) = repository::fail_import(&db, import_id, &message).await {
                    tracing::error!(%import_id, "could not record failed import: {e}");
                }
            }
        }
    }

    /// Returns a user-facing error message on failure.
    async fn process(&self) -> Result<Uuid, String> {
        let _slot = IMPORT_SLOT
            .acquire()
            .await
            .map_err(|_| "Import queue unavailable".to_string())?;

        let bytes = tokio::fs::read(&self.file.0)
            .await
            .map_err(|e| internal(format!("reading upload: {e}")))?;

        let parsed = tokio::task::spawn_blocking(move || -> Result<_, ImportError> {
            let ocr = Ocr::global().map_err(|e| ImportError::new(e.to_string()))?;
            import::parse(&bytes, ocr)
        })
        .await
        .map_err(|e| internal(format!("import task panicked: {e}")))?
        .map_err(|e| e.0)?;

        let tz = self.state.config.bookeo.timezone;
        let db = &self.state.db;
        if let Some(played) = repository::existing_game(db, &self.sha256, Some(&parsed.fingerprint))
            .await
            .map_err(internal)?
        {
            return Err(already_imported(played, tz).to_string());
        }

        // Match names to playernames (lowercase). Unknown names are kept, unlinked, and reported.
        let wanted: Vec<String> = parsed
            .players
            .iter()
            .map(|p| p.row.name.trim().to_lowercase())
            .collect();
        let found = repository::players_by_playername(db, &wanted)
            .await
            .map_err(internal)?;
        let player_ids: Vec<Option<Uuid>> = wanted
            .iter()
            .map(|n| found.iter().find(|(_, pn)| pn == n).map(|(id, _)| *id))
            .collect();
        let (mut matched, mut unmatched) = (Vec::new(), Vec::new());
        for (p, id) in parsed.players.iter().zip(&player_ids) {
            if id.is_some() {
                matched.push(p.row.name.to_lowercase());
            } else {
                unmatched.push(p.row.name.clone());
            }
        }

        let started_at = to_utc(parsed.summary.started_at, tz)?;
        let ended_at = to_utc(parsed.summary.ended_at, tz)?;

        match repository::save_game(
            db,
            self.import_id,
            &parsed,
            started_at,
            ended_at,
            &self.sha256,
            &self.filename,
            self.admin_id,
            &player_ids,
            &matched,
            &unmatched,
        )
        .await
        .map_err(internal)?
        {
            SaveOutcome::Saved(game_id) => Ok(game_id),
            SaveOutcome::Duplicate => Err(already_imported(started_at, tz).to_string()),
        }
    }
}

/// Deletes the uploaded file when dropped, so it never outlives its import.
struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.0)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::error!(path = %self.0.display(), "could not delete uploaded file: {e}");
        }
    }
}

async fn write_private(dir: &Path, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    tokio::fs::create_dir_all(dir).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).await?;
    }
    tokio::fs::write(path, bytes).await
}

fn to_utc(local: NaiveDateTime, tz: Tz) -> Result<DateTime<Utc>, String> {
    tz.from_local_datetime(&local)
        .earliest()
        .map(|t| t.with_timezone(&Utc))
        .ok_or_else(|| format!("The game time {local} doesn't exist in {}", tz.name()))
}

fn already_imported(played: DateTime<Utc>, tz: Tz) -> AppError {
    AppError::Conflict(format!(
        "This game was already imported (played {})",
        played.with_timezone(&tz).format("%d %b %Y, %H:%M")
    ))
}

/// Logs the detail, returns a generic message.
fn internal(e: impl std::fmt::Display) -> String {
    tracing::error!("stat import internal error: {e}");
    "Something went wrong while saving the statistics. Nothing was saved; please try again.".into()
}

/// Keeps a display-safe base name: no paths, no control characters, bounded length.
fn sanitize_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let clean: String = base.chars().filter(|c| !c.is_control()).take(200).collect();
    if clean.trim().is_empty() {
        "upload.pdf".into()
    } else {
        clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_filenames() {
        assert_eq!(
            sanitize_filename("Total_statistic_17:43.pdf"),
            "Total_statistic_17:43.pdf"
        );
        assert_eq!(sanitize_filename("../../etc/passwd.pdf"), "passwd.pdf");
        assert_eq!(sanitize_filename("C:\\x\\a.pdf"), "a.pdf");
        assert_eq!(sanitize_filename("a\nb.pdf"), "ab.pdf");
        assert_eq!(sanitize_filename(""), "upload.pdf");
    }

    #[test]
    fn temp_file_is_deleted_on_drop() {
        let path = std::env::temp_dir().join(format!("havoc-test-{}.pdf", Uuid::new_v4()));
        std::fs::write(&path, b"%PDF-").unwrap();
        drop(TempFile(path.clone()));
        assert!(!path.exists());
    }

    #[test]
    fn converts_local_game_time() {
        let local: NaiveDateTime = "2026-07-28T17:42:00".parse().unwrap();
        let utc = to_utc(local, chrono_tz::Asia::Qatar).unwrap();
        assert_eq!(utc.to_rfc3339(), "2026-07-28T14:42:00+00:00");
    }
}
