use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

use crate::error::{AppError, AppResult};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_and_verify_roundtrip() {
        let h = hash("password123".into()).await.unwrap();
        assert!(verify("password123".into(), h.clone()).await.unwrap());
        assert!(!verify("wrong".into(), h).await.unwrap());
    }
}
