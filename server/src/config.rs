use std::env;

use chrono_tz::Tz;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub jwt_secret: String,
    pub jwt_ttl_hours: i64,
    pub cors_origins: Vec<String>,
    pub cookie_secure: bool,
    pub bookeo: BookeoConfig,
}

#[derive(Clone, Debug)]
pub struct BookeoConfig {
    pub api_key: String,
    pub secret_key: String,
    pub base_url: String,
    /// The business's local timezone: "today" and date ranges are evaluated in it.
    pub timezone: Tz,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 characters".into());
        }

        Ok(Self {
            database_url: required("DATABASE_URL")?,
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into()),
            jwt_secret,
            jwt_ttl_hours: env::var("JWT_TTL_HOURS")
                .ok()
                .map(|v| v.parse().map_err(|_| "JWT_TTL_HOURS must be an integer"))
                .transpose()?
                .unwrap_or(168),
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            cookie_secure: env::var("COOKIE_SECURE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            bookeo: BookeoConfig {
                api_key: required("BOOKEO_API_KEY")?,
                secret_key: required("BOOKEO_SECRET_KEY")?,
                base_url: env::var("BOOKEO_BASE_URL")
                    .unwrap_or_else(|_| "https://api.bookeo.com/v2".into())
                    .trim_end_matches('/')
                    .to_string(),
                timezone: env::var("BOOKEO_TIMEZONE")
                    .unwrap_or_else(|_| "Asia/Qatar".into())
                    .parse()
                    .map_err(|_| "BOOKEO_TIMEZONE must be an IANA timezone, e.g. Asia/Qatar")?,
            },
        })
    }
}

fn required(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("{key} must be set"))
}
