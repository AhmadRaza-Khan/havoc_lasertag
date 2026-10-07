//! Transactional email through SMTP2GO's HTTP API (https://developers.smtp2go.com).
//! Without configuration, emails are logged instead of sent.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::EmailConfig;

pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: String,
}

pub struct Mailer {
    http: reqwest::Client,
    config: Option<EmailConfig>,
}

impl Mailer {
    pub fn new(config: Option<&EmailConfig>) -> Result<Self, reqwest::Error> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()?,
            config: config.cloned(),
        })
    }

    /// Sends in the background: a failed email is logged and never fails the caller.
    pub fn send_later(self: &std::sync::Arc<Self>, email: Email) {
        let mailer = self.clone();
        tokio::spawn(async move {
            if let Err(e) = mailer.send(&email).await {
                tracing::error!(to = %email.to, subject = %email.subject, "email not sent: {e}");
            }
        });
    }

    pub async fn send(&self, email: &Email) -> Result<(), String> {
        let Some(config) = &self.config else {
            tracing::info!(to = %email.to, subject = %email.subject, "email not configured; would send:\n{}", email.text);
            return Ok(());
        };
        let res = self
            .http
            .post(format!("{}/email/send", config.base_url))
            .header("X-Smtp2go-Api-Key", &config.api_key)
            .json(&SendRequest {
                sender: &config.from,
                to: [&email.to],
                subject: &email.subject,
                text_body: &email.text,
                html_body: &email.html,
            })
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        let parsed: Option<SendResponse> = serde_json::from_str(&body).ok();
        match parsed.and_then(|r| r.data) {
            Some(d) if status.is_success() && d.succeeded > 0 => {
                tracing::info!(to = %email.to, subject = %email.subject, "email sent");
                Ok(())
            }
            _ => Err(format!(
                "{status}: {}",
                body.chars().take(300).collect::<String>()
            )),
        }
    }
}

#[derive(Serialize)]
struct SendRequest<'a> {
    sender: &'a str,
    to: [&'a str; 1],
    subject: &'a str,
    text_body: &'a str,
    html_body: &'a str,
}

#[derive(Deserialize)]
struct SendResponse {
    data: Option<SendData>,
}

#[derive(Deserialize)]
struct SendData {
    #[serde(default)]
    succeeded: u32,
}
