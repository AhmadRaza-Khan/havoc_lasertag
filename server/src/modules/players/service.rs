use sqlx::PgPool;
use uuid::Uuid;

use super::{
    dto::{ListQuery, PlayerInput, SignupInput},
    model::Player,
    repository,
};
use crate::{
    error::{AppError, AppResult},
    shared::{
        pagination::{PageParams, Paginated, like_pattern},
        password,
        validation::{normalize_handle, validate_handle},
    },
};

pub async fn list(db: &PgPool, query: ListQuery) -> AppResult<Paginated<Player>> {
    let params = PageParams {
        page: query.page,
        per_page: query.per_page,
    };
    let pattern = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(like_pattern);

    let (items, total) =
        repository::list(db, pattern.as_deref(), params.per_page(), params.offset()).await?;
    Ok(Paginated::new(items, total, &params))
}

pub async fn get(db: &PgPool, id: Uuid) -> AppResult<Player> {
    repository::find_by_id(db, id).await?.ok_or_else(not_found)
}

pub async fn create(db: &PgPool, input: PlayerInput) -> AppResult<Player> {
    let mut player = input.validate(true)?;
    let hash = password::hash(player.password.take().unwrap_or_default()).await?;

    let created = repository::create(db, &player, &hash)
        .await
        .map_err(map_unique_violation)?;
    tracing::info!(player_id = %created.id, playername = %created.playername, "player created");
    Ok(created)
}

/// A player signing up in the app: like an admin-created player, but email and phone are required.
pub async fn signup(db: &PgPool, input: SignupInput) -> AppResult<Player> {
    let required = |label: &str, v: &str| {
        if v.trim().is_empty() {
            Err(AppError::BadRequest(format!("{label} is required")))
        } else {
            Ok(())
        }
    };
    required("Email", &input.email)?;
    required("Phone number", &input.phone)?;
    create(
        db,
        PlayerInput {
            full_name: input.full_name,
            playername: input.playername,
            phone: Some(input.phone),
            email: Some(input.email),
            password: Some(input.password),
        },
    )
    .await
}

/// Whether a playername can be taken: `Err` with the reason if it's invalid or in use.
pub async fn check_playername(db: &PgPool, raw: &str) -> AppResult<String> {
    let playername = normalize_handle(raw);
    validate_handle("Playername", &playername)?;
    if repository::find_credentials(db, &playername)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("Playername is already taken".into()));
    }
    Ok(playername)
}

pub async fn update(db: &PgPool, id: Uuid, input: PlayerInput) -> AppResult<Player> {
    if id == super::HAVOC_ADMIN_ID {
        return Err(not_found());
    }
    let mut player = input.validate(false)?;
    let hash = match player.password.take() {
        Some(p) => Some(password::hash(p).await?),
        None => None,
    };

    repository::update(db, id, &player, hash.as_deref())
        .await
        .map_err(map_unique_violation)?
        .ok_or_else(not_found)
}

pub async fn delete(db: &PgPool, id: Uuid) -> AppResult<()> {
    // The Havoc Admin chat account stays.
    if id == super::HAVOC_ADMIN_ID {
        return Err(not_found());
    }
    if !repository::delete(db, id).await? {
        return Err(not_found());
    }
    tracing::info!(player_id = %id, "player deleted");
    Ok(())
}

fn not_found() -> AppError {
    AppError::NotFound("Player not found".into())
}

fn map_unique_violation(e: sqlx::Error) -> AppError {
    match &e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            AppError::Conflict("Playername is already taken".into())
        }
        _ => AppError::Database(e),
    }
}
