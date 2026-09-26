use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    modules::users::PublicUser,
    shared::validation::{normalize_handle, required_text, validate_handle, validate_password},
};

#[derive(Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub name: String,
    pub password: String,
}

impl SignupRequest {
    /// Validates the request and returns it with normalized fields.
    pub fn validated(self) -> AppResult<Self> {
        let username = normalize_handle(&self.username);
        validate_handle("Username", &username)?;
        let name = required_text("Name", &self.name)?;
        validate_password(&self.password)?;

        Ok(Self {
            username,
            name,
            password: self.password,
        })
    }
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    /// Returned for non-browser clients (mobile). The web client relies on the httpOnly cookie.
    pub token: String,
    pub user: PublicUser,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(username: &str, name: &str, password: &str) -> SignupRequest {
        SignupRequest {
            username: username.into(),
            name: name.into(),
            password: password.into(),
        }
    }

    #[test]
    fn normalizes_username() {
        let r = req("  Admin_1 ", " Admin ", "password123")
            .validated()
            .unwrap();
        assert_eq!(r.username, "admin_1");
        assert_eq!(r.name, "Admin");
    }

    #[test]
    fn rejects_short_password_and_empty_name() {
        assert!(req("admin", "Name", "short").validated().is_err());
        assert!(req("admin", "   ", "password123").validated().is_err());
    }
}
