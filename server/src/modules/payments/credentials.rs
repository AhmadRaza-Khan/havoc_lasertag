//! SkipCash credentials, entered by admins in Settings. Saving them replaces the live client,
//! so payments switch over without a restart. Secrets are never sent back to the browser.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    integrations::skipcash::{NewPayment, SkipCashClient, SkipCashConfig},
    state::AppState,
};

/// Which SkipCash servers the credentials belong to (each has its own merchant portal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum Environment {
    Sandbox,
    Production,
}

impl Environment {
    fn base_url(self) -> &'static str {
        match self {
            Environment::Sandbox => "https://skipcashtest.azurewebsites.net",
            Environment::Production => "https://api.skipcash.app",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Environment::Sandbox => "sandbox",
            Environment::Production => "production",
        }
    }
}

#[derive(FromRow)]
struct Stored {
    environment: Environment,
    client_id: String,
    key_id: String,
    key_secret: String,
    webhook_key: String,
    updated_at: DateTime<Utc>,
}

/// What admins see: the IDs, and only whether the secrets are saved.
#[derive(Debug, Serialize)]
pub struct CredentialsView {
    pub configured: bool,
    pub environment: Option<Environment>,
    pub client_id: Option<String>,
    pub key_id: Option<String>,
    pub key_secret_set: bool,
    pub webhook_key_set: bool,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Empty secrets keep the saved ones, so admins can change an ID without retyping them.
#[derive(Debug, Deserialize)]
pub struct CredentialsInput {
    pub environment: Environment,
    pub client_id: String,
    pub key_id: String,
    #[serde(default)]
    pub key_secret: Option<String>,
    #[serde(default)]
    pub webhook_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TestResult {
    pub ok: bool,
    pub checks: Vec<Check>,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub name: &'static str,
    pub ok: bool,
    pub message: String,
}

/// At startup: uses the saved credentials, if any.
pub async fn load(state: &AppState) -> AppResult<()> {
    if let Some(s) = stored(&state.db).await? {
        state.set_skipcash(Some(client(&config(state, &s))?));
        tracing::info!(
            environment = s.environment.as_str(),
            "skipcash credentials loaded"
        );
    }
    Ok(())
}

pub async fn view(state: &AppState) -> AppResult<CredentialsView> {
    Ok(to_view(stored(&state.db).await?))
}

pub async fn save(state: &AppState, input: CredentialsInput) -> AppResult<CredentialsView> {
    let new = resolve(input, stored(&state.db).await?)?;
    let client = client(&config(state, &new))?;
    let saved: Stored = sqlx::query_as(
        "INSERT INTO skipcash_credentials (environment, client_id, key_id, key_secret, webhook_key)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (id) DO UPDATE SET environment = $1, client_id = $2, key_id = $3,
             key_secret = $4, webhook_key = $5, updated_at = now()
         RETURNING environment, client_id, key_id, key_secret, webhook_key, updated_at",
    )
    .bind(new.environment)
    .bind(&new.client_id)
    .bind(&new.key_id)
    .bind(&new.key_secret)
    .bind(&new.webhook_key)
    .fetch_one(&state.db)
    .await?;
    state.set_skipcash(Some(client));
    tracing::info!(
        environment = saved.environment.as_str(),
        "skipcash credentials saved"
    );
    Ok(to_view(Some(saved)))
}

/// Turns online payment off.
pub async fn remove(state: &AppState) -> AppResult<CredentialsView> {
    sqlx::query("DELETE FROM skipcash_credentials")
        .execute(&state.db)
        .await?;
    state.set_skipcash(None);
    tracing::info!("skipcash credentials removed: online payment is off");
    Ok(to_view(None))
}

/// Tries the credentials (as entered, or as saved where left empty) against SkipCash: creates
/// a 1.00 QAR test payment, which checks the key ID and secret, then reads it back with the
/// client ID. Nobody pays it; SkipCash cancels it within an hour. The webhook key can only be
/// checked by SkipCash calling us.
pub async fn test(state: &AppState, input: CredentialsInput) -> AppResult<TestResult> {
    let creds = resolve(input, stored(&state.db).await?)?;
    let client = client(&config(state, &creds))?;
    let reference = format!("conn-test-{}", Uuid::new_v4().simple());

    let created = client
        .create_payment(&NewPayment {
            amount: "1.00",
            first_name: "Havoc",
            last_name: "Connection test",
            phone: None,
            email: Some("connection-test@example.com"),
            transaction_id: &reference,
            custom1: Some("Connection test (not a booking)"),
        })
        .await;
    let mut checks = vec![Check {
        name: "Key ID and key secret",
        ok: created.is_ok(),
        message: match &created {
            Ok(_) => "Accepted: SkipCash created a 1.00 QAR test payment (it's never charged and \
                      is canceled within an hour)."
                .into(),
            Err(e) => format!("Refused: {e}"),
        },
    }];
    checks.push(match &created {
        Ok(c) => match client.payment(&c.id).await {
            Ok(_) => Check {
                name: "Client ID",
                ok: true,
                message: "Accepted: the test payment's status was read back.".into(),
            },
            Err(e) => Check {
                name: "Client ID",
                ok: false,
                message: format!("Refused: {e}"),
            },
        },
        Err(_) => Check {
            name: "Client ID",
            ok: false,
            message: "Not checked: needs a working key first.".into(),
        },
    });
    let ok = checks.iter().all(|c| c.ok);
    tracing::info!(
        environment = creds.environment.as_str(),
        ok,
        "skipcash connection test"
    );
    Ok(TestResult { ok, checks })
}

async fn stored(db: &PgPool) -> sqlx::Result<Option<Stored>> {
    sqlx::query_as(
        "SELECT environment, client_id, key_id, key_secret, webhook_key, updated_at
         FROM skipcash_credentials",
    )
    .fetch_optional(db)
    .await
}

/// The input, trimmed, with empty secrets taken from the saved credentials.
fn resolve(input: CredentialsInput, saved: Option<Stored>) -> AppResult<Stored> {
    let client_id = input.client_id.trim().to_string();
    let key_id = input.key_id.trim().to_string();
    if client_id.is_empty() || key_id.is_empty() {
        return Err(AppError::BadRequest(
            "Enter the client ID and the key ID".into(),
        ));
    }
    let secret = |entered: Option<String>, saved: Option<String>, label: &str| {
        entered
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .or(saved)
            .ok_or_else(|| AppError::BadRequest(format!("Enter the {label}")))
    };
    let (saved_secret, saved_webhook) = saved
        .map(|s| (Some(s.key_secret), Some(s.webhook_key)))
        .unwrap_or_default();
    Ok(Stored {
        environment: input.environment,
        client_id,
        key_id,
        key_secret: secret(input.key_secret, saved_secret, "key secret")?,
        webhook_key: secret(input.webhook_key, saved_webhook, "webhook key")?,
        updated_at: Utc::now(),
    })
}

fn config(state: &AppState, s: &Stored) -> SkipCashConfig {
    SkipCashConfig {
        base_url: state
            .config
            .skipcash_base_url
            .clone()
            .unwrap_or_else(|| s.environment.base_url().into()),
        client_id: s.client_id.clone(),
        key_id: s.key_id.clone(),
        key_secret: s.key_secret.clone(),
        webhook_key: s.webhook_key.clone(),
    }
}

fn client(config: &SkipCashConfig) -> AppResult<SkipCashClient> {
    SkipCashClient::new(config).map_err(|e| AppError::Internal(format!("http client: {e}")))
}

fn to_view(s: Option<Stored>) -> CredentialsView {
    CredentialsView {
        configured: s.is_some(),
        environment: s.as_ref().map(|s| s.environment),
        client_id: s.as_ref().map(|s| s.client_id.clone()),
        key_id: s.as_ref().map(|s| s.key_id.clone()),
        key_secret_set: s.is_some(),
        webhook_key_set: s.is_some(),
        updated_at: s.map(|s| s.updated_at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(secret: Option<&str>) -> CredentialsInput {
        CredentialsInput {
            environment: Environment::Sandbox,
            client_id: " client ".into(),
            key_id: "key".into(),
            key_secret: secret.map(Into::into),
            webhook_key: Some(String::new()),
        }
    }

    fn saved() -> Stored {
        Stored {
            environment: Environment::Production,
            client_id: "old-client".into(),
            key_id: "old-key".into(),
            key_secret: "old-secret".into(),
            webhook_key: "old-webhook".into(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn empty_secrets_keep_the_saved_ones() {
        let r = resolve(input(None), Some(saved())).unwrap();
        assert_eq!(r.client_id, "client");
        assert_eq!(r.key_secret, "old-secret");
        assert_eq!(r.webhook_key, "old-webhook");
        let r = resolve(input(Some(" new ")), Some(saved())).unwrap();
        assert_eq!(r.key_secret, "new");
    }

    #[test]
    fn first_save_needs_every_secret() {
        assert!(resolve(input(Some("s")), None).is_err());
    }
}
