//! Expo Push Service (https://docs.expo.dev/push-notifications/sending-notifications/):
//! one HTTP call delivers to both FCM (Android) and APNs (iOS) for Expo push tokens.

use std::time::Duration;

use serde::{Deserialize, Serialize};

const DEFAULT_URL: &str = "https://exp.host/--/api/v2/push/send";
/// Expo accepts at most 100 messages per request.
const MAX_BATCH: usize = 100;

#[derive(Debug, Clone, Serialize)]
pub struct PushMessage {
    pub to: String,
    pub title: String,
    pub body: String,
    /// Delivered to the app with the notification (e.g. which chat to open).
    pub data: serde_json::Value,
    pub sound: &'static str,
    /// Android notification channel created by the app.
    #[serde(rename = "channelId")]
    pub channel_id: &'static str,
    /// iOS: group notifications by conversation.
    #[serde(rename = "threadId", skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

/// Outcome per token, in the order sent.
#[derive(Debug, PartialEq)]
pub enum Delivery {
    Accepted,
    /// The app was uninstalled or the token expired: stop using it.
    DeviceNotRegistered,
    Failed(String),
}

pub struct ExpoPush {
    http: reqwest::Client,
    url: String,
    access_token: Option<String>,
}

impl ExpoPush {
    pub fn new(url: Option<String>, access_token: Option<String>) -> Result<Self, reqwest::Error> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()?,
            url: url.unwrap_or_else(|| DEFAULT_URL.into()),
            access_token,
        })
    }

    /// Sends the messages; returns one result per message. A transport failure fails them all.
    pub async fn send(&self, messages: &[PushMessage]) -> Vec<Delivery> {
        let mut out = Vec::with_capacity(messages.len());
        for chunk in messages.chunks(MAX_BATCH) {
            out.extend(self.send_chunk(chunk).await);
        }
        out
    }

    async fn send_chunk(&self, messages: &[PushMessage]) -> Vec<Delivery> {
        let failed = |why: String| {
            messages
                .iter()
                .map(|_| Delivery::Failed(why.clone()))
                .collect()
        };
        let mut req = self.http.post(&self.url).json(messages);
        if let Some(t) = &self.access_token {
            req = req.bearer_auth(t);
        }
        let res = match req.send().await {
            Ok(r) => r,
            Err(e) => return failed(format!("request failed: {e}")),
        };
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        let Ok(parsed) = serde_json::from_str::<Response>(&text) else {
            return failed(format!(
                "{status}: {}",
                text.chars().take(200).collect::<String>()
            ));
        };
        if parsed.data.len() != messages.len() {
            return failed(format!(
                "{status}: {} tickets for {} messages",
                parsed.data.len(),
                messages.len()
            ));
        }
        parsed.data.into_iter().map(Ticket::delivery).collect()
    }
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    data: Vec<Ticket>,
}

#[derive(Deserialize)]
struct Ticket {
    status: String,
    message: Option<String>,
    details: Option<TicketDetails>,
}

#[derive(Deserialize)]
struct TicketDetails {
    error: Option<String>,
}

impl Ticket {
    fn delivery(self) -> Delivery {
        if self.status == "ok" {
            return Delivery::Accepted;
        }
        match self.details.and_then(|d| d.error).as_deref() {
            Some("DeviceNotRegistered") => Delivery::DeviceNotRegistered,
            other => Delivery::Failed(
                other
                    .map(str::to_owned)
                    .or(self.message)
                    .unwrap_or_else(|| "unknown error".into()),
            ),
        }
    }
}

/// Expo push tokens look like `ExponentPushToken[xxxxxxxx]` (or `ExpoPushToken[...]`).
pub fn is_expo_token(token: &str) -> bool {
    let inner = token
        .strip_prefix("ExponentPushToken[")
        .or_else(|| token.strip_prefix("ExpoPushToken["))
        .and_then(|t| t.strip_suffix(']'));
    inner.is_some_and(|t| {
        !t.is_empty()
            && t.len() <= 200
            && t.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_expo_tokens() {
        assert!(is_expo_token("ExponentPushToken[xxxxxxxxxxxxxxxxxxxxxx]"));
        assert!(is_expo_token("ExpoPushToken[abc-DEF_123]"));
        assert!(!is_expo_token("ExponentPushToken[]"));
        assert!(!is_expo_token("fcm-raw-token"));
        assert!(!is_expo_token("ExponentPushToken[a b]"));
    }

    #[test]
    fn reads_tickets() {
        let r: Response = serde_json::from_str(
            r#"{"data":[{"status":"ok","id":"1"},{"status":"error","message":"gone","details":{"error":"DeviceNotRegistered"}},{"status":"error","message":"too big","details":{"error":"MessageTooBig"}}]}"#,
        )
        .unwrap();
        let d: Vec<_> = r.data.into_iter().map(Ticket::delivery).collect();
        assert_eq!(
            d,
            [
                Delivery::Accepted,
                Delivery::DeviceNotRegistered,
                Delivery::Failed("MessageTooBig".into())
            ]
        );
    }
}
