use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// Who a token was issued to. Keeps future player (mobile) tokens from passing admin checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    Admin,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub kind: TokenKind,
    pub iat: i64,
    pub exp: i64,
}

pub fn create_token(
    subject: Uuid,
    kind: TokenKind,
    secret: &str,
    ttl_hours: i64,
) -> AppResult<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: subject,
        kind,
        iat: now.timestamp(),
        exp: (now + Duration::hours(ttl_hours)).timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("failed to sign token: {e}")))
}

pub fn verify_token(token: &str, secret: &str) -> AppResult<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized("Invalid or expired token".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret-that-is-long-enough-000000";

    #[test]
    fn roundtrip() {
        let id = Uuid::new_v4();
        let token = create_token(id, TokenKind::Admin, SECRET, 1).unwrap();
        let claims = verify_token(&token, SECRET).unwrap();
        assert_eq!((claims.sub, claims.kind), (id, TokenKind::Admin));
    }

    #[test]
    fn rejects_wrong_secret_and_expired() {
        let token = create_token(Uuid::new_v4(), TokenKind::Admin, SECRET, 1).unwrap();
        assert!(verify_token(&token, "another-secret-that-is-long-enough-00").is_err());

        let expired = create_token(Uuid::new_v4(), TokenKind::Admin, SECRET, -1).unwrap();
        assert!(verify_token(&expired, SECRET).is_err());
    }
}
