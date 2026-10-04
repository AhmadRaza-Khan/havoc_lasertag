use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

use tokio::sync::OnceCell;

use crate::error::{AppError, AppResult};

/// Hash of a random value, checked against when the account doesn't exist so that login
/// takes the same time either way (no username enumeration by timing).
static DUMMY_HASH: OnceCell<String> = OnceCell::const_new();

/// Argon2 is deliberately slow, so run it off the async runtime.
pub async fn hash(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|h| h.to_string())
            .map_err(|e| AppError::Internal(format!("password hashing failed: {e}")))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

pub async fn verify(password: String, hash: String) -> AppResult<bool> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&hash)
            .map_err(|e| AppError::Internal(format!("invalid stored password hash: {e}")))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

/// Verifies against `hash`, or against a dummy hash when the account wasn't found. Always
/// false for a missing account, but costs the same as a real check.
pub async fn verify_or_dummy(password: String, hash: Option<String>) -> AppResult<bool> {
    match hash {
        Some(hash) => verify(password, hash).await,
        None => {
            let dummy = DUMMY_HASH.get_or_try_init(hash_random).await?.clone();
            verify(password, dummy).await?;
            Ok(false)
        }
    }
}

async fn hash_random() -> AppResult<String> {
    hash(uuid::Uuid::new_v4().to_string()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_and_verify_roundtrip() {
        let h = hash("password123".into()).await.unwrap();
        assert!(verify("password123".into(), h.clone()).await.unwrap());
        assert!(!verify("wrong".into(), h).await.unwrap());
    }

    #[tokio::test]
    async fn missing_account_never_verifies() {
        assert!(!verify_or_dummy("anything".into(), None).await.unwrap());
        let h = hash("password123".into()).await.unwrap();
        assert!(
            verify_or_dummy("password123".into(), Some(h))
                .await
                .unwrap()
        );
    }
}
