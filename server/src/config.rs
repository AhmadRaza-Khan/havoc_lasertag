use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub jwt_secret: String,
    pub jwt_ttl_hours: i64,
    pub cors_origins: Vec<String>,
    pub cookie_secure: bool,
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
        })
    }
}

fn required(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("{key} must be set"))
}
