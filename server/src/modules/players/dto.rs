use serde::Deserialize;

use crate::{
    error::AppResult,
    shared::validation::{
        normalize_handle, optional_text, required_text, validate_email, validate_handle,
        validate_password, validate_phone,
    },
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Matches playername or full name, case-insensitive.
    pub search: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Body for both create (POST) and update (PUT).
/// On update, a missing/blank `password` keeps the current one.
#[derive(Debug, Deserialize)]
pub struct PlayerInput {
    pub full_name: String,
    pub playername: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
}

/// Validated, normalized player fields ready to persist.
#[derive(Debug)]
pub struct ValidPlayer {
    pub full_name: String,
    pub playername: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
}

impl PlayerInput {
    pub fn validate(self, password_required: bool) -> AppResult<ValidPlayer> {
        let full_name = required_text("Full name", &self.full_name)?;

        let playername = normalize_handle(&self.playername);
        validate_handle("Playername", &playername)?;

        let phone = optional_text(self.phone);
        if let Some(p) = &phone {
            validate_phone(p)?;
        }

        let email = optional_text(self.email).map(|e| e.to_lowercase());
        if let Some(e) = &email {
            validate_email(e)?;
        }

        let password = self.password.filter(|p| !p.is_empty());
        match &password {
            Some(p) => validate_password(p)?,
            None if password_required => validate_password("")?,
            None => {}
        }

        Ok(ValidPlayer {
            full_name,
            playername,
            phone,
            email,
            password,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(password: Option<&str>) -> PlayerInput {
        PlayerInput {
            full_name: " Jane Doe ".into(),
            playername: " Jane_D ".into(),
            phone: Some("  ".into()),
            email: Some(" Jane@Example.com ".into()),
            password: password.map(Into::into),
        }
    }

    #[test]
    fn normalizes_fields() {
        let p = input(Some("password123")).validate(true).unwrap();
        assert_eq!(p.full_name, "Jane Doe");
        assert_eq!(p.playername, "jane_d");
        assert_eq!(p.phone, None);
        assert_eq!(p.email.as_deref(), Some("jane@example.com"));
    }

    #[test]
    fn password_required_only_on_create() {
        assert!(input(None).validate(true).is_err());
        assert!(input(Some("")).validate(true).is_err());
        assert!(input(None).validate(false).unwrap().password.is_none());
        assert!(input(Some("short")).validate(false).is_err());
    }
}
