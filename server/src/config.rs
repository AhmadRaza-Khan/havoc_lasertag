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
    /// QNB payment gateway. `None` turns online payment off: app bookings have nothing to pay.
    pub qnb: Option<QnbConfig>,
    /// SMTP2GO email. `None` logs emails instead of sending them.
    pub email: Option<EmailConfig>,
    /// Key for the website's read-only leaderboard API (the WordPress plugin). `None` turns
    /// that API off.
    pub website_api_key: Option<String>,
}

#[derive(Clone, Debug)]
pub struct QnbConfig {
    /// e.g. https://ecommerce-uat.qnb.com (test) — paths like /ipg/rest/v1/... are appended.
    pub base_url: String,
    pub user_id: String,
    pub merchant_id: String,
    /// API password ("Merchant API Details" in the QNB merchant console).
    pub merchant_pwd: String,
    /// API key ("Merchant API Details" in the QNB merchant console).
    pub merchant_key: String,
}

#[derive(Clone, Debug)]
pub struct EmailConfig {
    pub api_key: String,
    /// Sender, e.g. `Havoc Laser Tag <no-reply@havoc.qa>`; the domain must be verified in SMTP2GO.
    pub from: String,
    pub base_url: String,
}

#[derive(Clone, Debug)]
pub struct BookeoConfig {
    pub api_key: String,
    pub secret_key: String,
    pub base_url: String,
    /// The business's local timezone: "today" and date ranges are evaluated in it.
    pub timezone: Tz,
    /// Public URL of our "booking canceled" webhook, e.g.
    /// `https://api.example.com/api/webhooks/bookeo/bookings-deleted`. When set, the server
    /// registers it with Bookeo at startup and accepts its calls; unset turns the webhook off.
    /// Leave it unset on local/dev servers: Bookeo keeps one webhook per API key, so a dev
    /// server registering its URL would take the webhook away from production.
    pub webhook_url: Option<String>,
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
                webhook_url: webhook_url()?,
            },
            qnb: qnb_config()?,
            website_api_key: website_api_key()?,
            email: email_config()?,
        })
    }
}

/// `BOOKEO_WEBHOOK_URL`, if set. Bookeo only calls https URLs.
fn webhook_url() -> Result<Option<String>, String> {
    let Some(url) = env::var("BOOKEO_WEBHOOK_URL")
        .ok()
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        return Ok(None);
    };
    if !url.starts_with("https://") {
        return Err("BOOKEO_WEBHOOK_URL must be an https URL (Bookeo only calls https)".into());
    }
    Ok(Some(url))
}

/// `WEBSITE_API_KEY`, if set: a long random secret shared with the website plugin.
fn website_api_key() -> Result<Option<String>, String> {
    let Some(key) = env::var("WEBSITE_API_KEY")
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
    else {
        return Ok(None);
    };
    if key.len() < 32 {
        return Err("WEBSITE_API_KEY must be at least 32 characters".into());
    }
    Ok(Some(key))
}

/// All of the QNB variables, or none of them.
fn qnb_config() -> Result<Option<QnbConfig>, String> {
    let keys = [
        "QNB_USER_ID",
        "QNB_MERCHANT_ID",
        "QNB_MERCHANT_PWD",
        "QNB_MERCHANT_KEY",
    ];
    if keys.iter().all(|k| env::var(k).is_err()) {
        return Ok(None);
    }
    Ok(Some(QnbConfig {
        base_url: env::var("QNB_BASE_URL")
            .unwrap_or_else(|_| "https://ecommerce-uat.qnb.com".into())
            .trim_end_matches('/')
            .to_string(),
        user_id: required(keys[0])?,
        merchant_id: required(keys[1])?,
        merchant_pwd: required(keys[2])?,
        merchant_key: required(keys[3])?,
    }))
}

fn email_config() -> Result<Option<EmailConfig>, String> {
    let Ok(api_key) = env::var("SMTP2GO_API_KEY") else {
        return Ok(None);
    };
    Ok(Some(EmailConfig {
        api_key,
        from: required("MAIL_FROM")?,
        base_url: env::var("SMTP2GO_BASE_URL")
            .unwrap_or_else(|_| "https://api.smtp2go.com/v3".into())
            .trim_end_matches('/')
            .to_string(),
    }))
}

fn required(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("{key} must be set"))
}
