//! Input normalization and validation shared by feature DTOs.

use crate::error::{AppError, AppResult};

pub const HANDLE_MIN: usize = 3;
pub const HANDLE_MAX: usize = 32;
pub const PASSWORD_MIN: usize = 8;

/// Handles (usernames, playernames) are case-insensitive: stored and looked up in lowercase.
pub fn normalize_handle(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Validates an already-normalized handle. `label` is used in error messages, e.g. "Username".
pub fn validate_handle(label: &str, value: &str) -> AppResult<()> {
    let len = value.chars().count();
    if !(HANDLE_MIN..=HANDLE_MAX).contains(&len) {
        return Err(AppError::BadRequest(format!(
            "{label} must be {HANDLE_MIN}-{HANDLE_MAX} characters"
        )));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        return Err(AppError::BadRequest(format!(
            "{label} may only contain letters, numbers, '_', '.' and '-'"
        )));
    }
    Ok(())
}

pub fn validate_password(password: &str) -> AppResult<()> {
    if password.chars().count() < PASSWORD_MIN {
        return Err(AppError::BadRequest(format!(
            "Password must be at least {PASSWORD_MIN} characters"
        )));
    }
    Ok(())
}

/// Trims a required text field and rejects it if empty.
pub fn required_text(label: &str, value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{label} is required")));
    }
    Ok(value.to_string())
}

/// Trims an optional text field; blank strings become `None`.
pub fn optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub fn validate_email(email: &str) -> AppResult<()> {
    let valid = email.len() <= 254
        && !email.contains(char::is_whitespace)
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if !valid {
        return Err(AppError::BadRequest("Invalid email address".into()));
    }
    Ok(())
}

pub fn validate_phone(phone: &str) -> AppResult<()> {
    let digits = phone.chars().filter(char::is_ascii_digit).count();
    let allowed = phone
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '-' | '(' | ')'));
    if !allowed || !(7..=15).contains(&digits) {
        return Err(AppError::BadRequest("Invalid phone number".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles() {
        assert_eq!(normalize_handle("  Admin_1 "), "admin_1");
        assert!(validate_handle("Username", "admin_1").is_ok());
        for bad in ["ab", "has space", "bad@char", &"x".repeat(33)] {
            assert!(validate_handle("Username", bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn emails() {
        assert!(validate_email("a@b.co").is_ok());
        for bad in ["nope", "@b.co", "a@b", "a b@c.de"] {
            assert!(validate_email(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn phones() {
        assert!(validate_phone("+1 (555) 123-4567").is_ok());
        for bad in ["123", "phone", "+1 555 abc 4567"] {
            assert!(validate_phone(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn optional_text_blank_is_none() {
        assert_eq!(optional_text(Some("  ".into())), None);
        assert_eq!(optional_text(Some(" x ".into())), Some("x".into()));
        assert_eq!(optional_text(None), None);
    }
}
