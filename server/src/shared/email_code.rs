//! 6-digit codes sent by email (password resets, email verification, account deletion). One
//! pending code per account and purpose; asking again replaces it. Codes are stored hashed,
//! expire after 15 minutes and stop working after 5 wrong tries.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppResult, integrations::email::Email, shared::password};

/// How long a code works.
pub const TTL_MINUTES: i64 = 15;
/// Wrong tries before the code stops working.
const MAX_ATTEMPTS: i32 = 5;
/// A new code isn't issued again within this many seconds (no email flooding).
const RESEND_AFTER_SECONDS: f64 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    PlayerReset,
    PlayerVerify,
    PlayerDelete,
    StaffReset,
}

impl Purpose {
    fn as_str(self) -> &'static str {
        match self {
            Purpose::PlayerReset => "player_reset",
            Purpose::PlayerVerify => "player_verify",
            Purpose::PlayerDelete => "player_delete",
            Purpose::StaffReset => "staff_reset",
        }
    }
}

/// A new code for `subject`, replacing any pending one. `None` if one was issued under a minute
/// ago (the earlier email is still on its way).
pub async fn issue(db: &PgPool, subject: Uuid, purpose: Purpose) -> AppResult<Option<String>> {
    let recent: Option<bool> = sqlx::query_scalar(
        "SELECT created_at > now() - make_interval(secs => $3)
         FROM email_codes WHERE subject_id = $1 AND purpose = $2",
    )
    .bind(subject)
    .bind(purpose.as_str())
    .bind(RESEND_AFTER_SECONDS)
    .fetch_optional(db)
    .await?;
    if recent == Some(true) {
        return Ok(None);
    }
    let code = new_code();
    let hash = password::hash(code.clone()).await?;
    sqlx::query(
        "INSERT INTO email_codes (subject_id, purpose, code_hash, expires_at) VALUES ($1, $2, $3, $4)
         ON CONFLICT (subject_id, purpose) DO UPDATE
             SET code_hash = $3, expires_at = $4, attempts = 0, created_at = now()",
    )
    .bind(subject)
    .bind(purpose.as_str())
    .bind(&hash)
    .bind(Utc::now() + Duration::minutes(TTL_MINUTES))
    .execute(db)
    .await?;
    Ok(Some(code))
}

/// Whether `subject` has a code that still works (not expired, tries left).
pub async fn has_pending(db: &PgPool, subject: Uuid, purpose: Purpose) -> AppResult<bool> {
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM email_codes
                        WHERE subject_id = $1 AND purpose = $2 AND expires_at > now() AND attempts < $3)",
    )
    .bind(subject)
    .bind(purpose.as_str())
    .bind(MAX_ATTEMPTS)
    .fetch_one(db)
    .await?;
    Ok(pending)
}

/// Whether `code` is the pending one for `subject`. A right code is used up; a wrong one counts
/// towards the limit. Always costs one hash check (no timing difference without a code).
pub async fn check(db: &PgPool, subject: Uuid, purpose: Purpose, code: &str) -> AppResult<bool> {
    let pending: Option<(String, i32)> = sqlx::query_as(
        "SELECT code_hash, attempts FROM email_codes
         WHERE subject_id = $1 AND purpose = $2 AND expires_at > now()",
    )
    .bind(subject)
    .bind(purpose.as_str())
    .fetch_optional(db)
    .await?;
    let code = code.trim().to_string();
    let Some((hash, attempts)) = pending else {
        password::verify_or_dummy(code, None).await?;
        return Ok(false);
    };
    if attempts >= MAX_ATTEMPTS || !password::verify(code, hash).await? {
        sqlx::query(
            "UPDATE email_codes SET attempts = attempts + 1 WHERE subject_id = $1 AND purpose = $2",
        )
        .bind(subject)
        .bind(purpose.as_str())
        .execute(db)
        .await?;
        return Ok(false);
    }
    sqlx::query("DELETE FROM email_codes WHERE subject_id = $1 AND purpose = $2")
        .bind(subject)
        .bind(purpose.as_str())
        .execute(db)
        .await?;
    Ok(true)
}

/// 6 random digits, from the system's secure random source (via UUID v4).
pub fn new_code() -> String {
    let bytes = Uuid::new_v4().into_bytes();
    let n = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % 1_000_000;
    format!("{n:06}")
}

/// The email carrying a code: "Hi {name}, {what}: 123456 …".
pub fn email(to: &str, name: &str, subject: &str, what: &str, code: &str) -> Email {
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let text = format!(
        "Hi {name},\n\n{what}: {code}\n\nIt works for {TTL_MINUTES} minutes. If you didn't ask for this, you can ignore this email.\n\nHavoc Tactical Laser Tag"
    );
    let html = format!(
        "<div style=\"font-family:Arial,sans-serif;font-size:15px;line-height:1.5\">\
         <p>Hi {},</p><p>{}:</p>\
         <p style=\"font-size:28px;font-weight:700;letter-spacing:6px\">{code}</p>\
         <p>It works for {TTL_MINUTES} minutes. If you didn't ask for this, you can ignore this email.</p>\
         <p style=\"color:#666\">Havoc Tactical Laser Tag</p></div>",
        esc(name),
        esc(what)
    );
    Email {
        to: to.to_string(),
        subject: subject.to_string(),
        text,
        html,
    }
}

/// "tom@example.com" → "t••@example.com", to say where a code went without showing the address.
pub fn hint(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            let first: String = local.chars().take(1).collect();
            format!("{first}••@{domain}")
        }
        None => "your email".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_six_digits() {
        for _ in 0..50 {
            let c = new_code();
            assert_eq!(c.len(), 6);
            assert!(c.chars().all(|ch| ch.is_ascii_digit()));
        }
    }

    #[test]
    fn email_has_the_code_and_escapes_the_name() {
        let e = email("a@b.c", "Tom <b>", "Subject", "Your code", "042137");
        assert!(e.text.contains("042137") && e.html.contains("042137"));
        assert!(e.html.contains("Tom &lt;b&gt;"));
    }

    #[test]
    fn hints_hide_the_address() {
        assert_eq!(hint("tom@example.com"), "t••@example.com");
        assert_eq!(hint("broken"), "your email");
    }
}
