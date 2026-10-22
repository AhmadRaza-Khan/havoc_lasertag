use std::time::Duration;

use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;

use super::signature::{data, sign};
use crate::error::AppError;

#[derive(Debug, thiserror::Error)]
pub enum SkipCashError {
    /// SkipCash refused the request (validation or business error in the body).
    #[error("{0}")]
    Rejected(String),
    /// Network failure, outage, bad credentials, unexpected response.
    #[error("{0}")]
    Unavailable(String),
}

impl From<SkipCashError> for AppError {
    fn from(e: SkipCashError) -> Self {
        match e {
            // Shown to the player, e.g. a missing email: they can act on it.
            SkipCashError::Rejected(m) => AppError::Unprocessable(format!("SkipCash: {m}")),
            SkipCashError::Unavailable(m) => AppError::BadGateway(format!("skipcash: {m}")),
        }
    }
}

type SkipCashResult<T> = Result<T, SkipCashError>;

/// The merchant's credentials (merchant portal → Credentials), entered by admins in Settings.
#[derive(Clone, Debug)]
pub struct SkipCashConfig {
    /// Production: https://api.skipcash.app · sandbox: https://skipcashtest.azurewebsites.net
    pub base_url: String,
    /// "Client ID"; authorizes reading a payment.
    pub client_id: String,
    /// The private key's ID (Credentials → keys table, "Id").
    pub key_id: String,
    /// The private key's secret; signs payment requests. Never sent to the app.
    pub key_secret: String,
    /// "Webhook Key"; SkipCash signs its webhook calls with it.
    pub webhook_key: String,
}

/// A payment to create. Text limits from the manual (§2.1) are applied here.
pub struct NewPayment<'a> {
    /// Decimal with a dot and at most 2 decimals, e.g. "150.00".
    pub amount: &'a str,
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub phone: Option<&'a str>,
    pub email: Option<&'a str>,
    /// Our order reference (max 40); comes back in webhooks.
    pub transaction_id: &'a str,
    /// Free text (max 50), e.g. the booking number.
    pub custom1: Option<&'a str>,
}

/// A created payment: SkipCash's id and the page to open.
#[derive(Debug)]
pub struct CreatedPayment {
    pub id: String,
    pub pay_url: String,
}

/// What SkipCash says about a payment (manual §2.2 `statusId`).
#[derive(Debug, PartialEq)]
pub enum PaymentState {
    /// 0 new, 1 pending: not paid (yet).
    Pending,
    /// 2 paid. `amount` as SkipCash reports it; `visa_id` is the card processor's reference.
    Paid {
        amount: String,
        currency: String,
        visa_id: Option<String>,
    },
    /// 3 canceled (unfinished payments are canceled after an hour): the page can't be paid.
    Canceled,
    /// 4 failed, 5 rejected.
    Failed,
    /// 6 refunded, 7 pending refund, 8 refund failed.
    Refunded,
}

impl PaymentState {
    pub fn from_status(
        status_id: i64,
        amount: String,
        currency: String,
        visa_id: Option<String>,
    ) -> Self {
        match status_id {
            2 => PaymentState::Paid {
                amount,
                currency,
                visa_id,
            },
            3 => PaymentState::Canceled,
            4 | 5 => PaymentState::Failed,
            6..=8 => PaymentState::Refunded,
            _ => PaymentState::Pending,
        }
    }
}

pub struct SkipCashClient {
    http: reqwest::Client,
    config: SkipCashConfig,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateBody<'a> {
    uid: String,
    key_id: &'a str,
    amount: &'a str,
    first_name: String,
    last_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    transaction_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom1: Option<String>,
}

/// Every SkipCash response is wrapped like this.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<T> {
    result_obj: Option<T>,
    #[serde(default)]
    has_error: bool,
    #[serde(default)]
    has_validation_error: bool,
    error_message: Option<String>,
    #[serde(default)]
    validation_errors: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaymentDetail {
    id: String,
    status_id: i64,
    pay_url: Option<String>,
    amount: Option<serde_json::Value>,
    currency: Option<String>,
    visa_id: Option<String>,
}

impl SkipCashClient {
    pub fn new(config: &SkipCashConfig) -> Result<Self, reqwest::Error> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(10))
                .build()?,
            config: config.clone(),
        })
    }

    /// The key SkipCash signs its webhook calls with.
    pub fn webhook_key(&self) -> &str {
        &self.config.webhook_key
    }

    /// `POST /api/v1/payments`, signed with our private key (manual §2.1).
    pub async fn create_payment(&self, p: &NewPayment<'_>) -> SkipCashResult<CreatedPayment> {
        let c = &self.config;
        let body = CreateBody {
            uid: Uuid::new_v4().to_string(),
            key_id: &c.key_id,
            amount: p.amount,
            first_name: clip(p.first_name, 60),
            last_name: clip(p.last_name, 60),
            phone: p
                .phone
                .map(|v| clip(&v.replace(' ', ""), 15))
                .filter(|v| !v.is_empty()),
            email: p.email.map(|v| clip(v, 255)).filter(|v| !v.is_empty()),
            transaction_id: clip(p.transaction_id, 40),
            custom1: p.custom1.map(|v| clip(v, 50)),
        };
        // The signed fields, in the manual's order (unused ones are empty and left out).
        let signed = data(&[
            ("Uid", Some(&body.uid)),
            ("KeyId", Some(body.key_id)),
            ("Amount", Some(body.amount)),
            ("FirstName", Some(&body.first_name)),
            ("LastName", Some(&body.last_name)),
            ("Phone", body.phone.as_deref()),
            ("Email", body.email.as_deref()),
            ("TransactionId", Some(&body.transaction_id)),
            ("Custom1", body.custom1.as_deref()),
        ]);
        let res = self
            .http
            .post(format!("{}/api/v1/payments", c.base_url))
            .header(AUTHORIZATION, sign(&c.key_secret, &signed))
            .json(&body)
            .send()
            .await
            .map_err(|e| SkipCashError::Unavailable(format!("create payment: {e}")))?;
        let detail: PaymentDetail = result(res).await?;
        let pay_url = detail
            .pay_url
            .filter(|u| !u.is_empty())
            .ok_or_else(|| SkipCashError::Unavailable("payment created without a payUrl".into()))?;
        Ok(CreatedPayment {
            id: detail.id,
            pay_url,
        })
    }

    /// `GET /api/v1/payments/{id}`, authorized with our client ID (manual §2.2).
    pub async fn payment(&self, id: &str) -> SkipCashResult<PaymentState> {
        let id = Uuid::parse_str(id)
            .map_err(|_| SkipCashError::Rejected("invalid payment id".into()))?;
        let res = self
            .http
            .get(format!("{}/api/v1/payments/{id}", self.config.base_url))
            .header(AUTHORIZATION, &self.config.client_id)
            .send()
            .await
            .map_err(|e| SkipCashError::Unavailable(format!("payment status: {e}")))?;
        let d: PaymentDetail = result(res).await?;
        let amount = match d.amount {
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::String(s)) => s,
            _ => String::new(),
        };
        Ok(PaymentState::from_status(
            d.status_id,
            amount,
            d.currency.unwrap_or_else(|| "QAR".into()),
            d.visa_id.filter(|v| !v.is_empty()),
        ))
    }
}

/// Unwraps SkipCash's envelope, turning its error fields into a `SkipCashError`.
async fn result<T: DeserializeOwned>(res: reqwest::Response) -> SkipCashResult<T> {
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| SkipCashError::Unavailable(format!("reading response: {e}")))?;
    let envelope: Envelope<T> = serde_json::from_str(&text).map_err(|_| {
        SkipCashError::Unavailable(format!(
            "unexpected response ({status}): {}",
            snippet(&text)
        ))
    })?;
    if envelope.has_error || envelope.has_validation_error {
        let detail = envelope
            .validation_errors
            .as_ref()
            .and_then(readable_errors)
            .or(envelope.error_message)
            .unwrap_or_else(|| format!("request refused ({status})"));
        return Err(
            if status.is_server_error() || status.as_u16() == 401 || status.as_u16() == 403 {
                SkipCashError::Unavailable(detail)
            } else {
                SkipCashError::Rejected(detail)
            },
        );
    }
    envelope
        .result_obj
        .ok_or_else(|| SkipCashError::Unavailable(format!("empty response ({status})")))
}

/// `{"Email":["required"]}` → "Email: required".
fn readable_errors(v: &serde_json::Value) -> Option<String> {
    let map = v.as_object()?;
    let parts: Vec<String> = map
        .iter()
        .map(|(field, msgs)| {
            let msgs = match msgs {
                serde_json::Value::Array(a) => a
                    .iter()
                    .filter_map(|m| m.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                other => other.as_str().unwrap_or_default().to_string(),
            };
            format!("{field}: {msgs}")
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join("; "))
}

fn clip(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

fn snippet(text: &str) -> String {
    text.chars().take(200).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_validation_errors_readable() {
        let v = serde_json::json!({"Email": ["required"], "Phone": ["too long", "invalid"]});
        assert_eq!(
            readable_errors(&v).as_deref(),
            Some("Email: required; Phone: too long, invalid")
        );
        assert_eq!(readable_errors(&serde_json::Value::Null), None);
    }

    #[test]
    fn maps_statuses() {
        let st = |id| PaymentState::from_status(id, "19.00".into(), "QAR".into(), None);
        assert_eq!(st(0), PaymentState::Pending);
        assert_eq!(st(1), PaymentState::Pending);
        assert_eq!(
            st(2),
            PaymentState::Paid {
                amount: "19.00".into(),
                currency: "QAR".into(),
                visa_id: None
            }
        );
        assert_eq!(st(3), PaymentState::Canceled);
        assert_eq!(st(4), PaymentState::Failed);
        assert_eq!(st(5), PaymentState::Failed);
        assert_eq!(st(6), PaymentState::Refunded);
    }

    #[test]
    fn reads_the_manuals_response() {
        let env: Envelope<PaymentDetail> = serde_json::from_str(
            r#"{"resultObj":{"id":"770bd18b-a505-468b-8de1-ac7f9555dc79","statusId":0,"created":"2021-06-30T13:33:52Z","payUrl":"https://skipcashtest.azurewebsites.net/pay/770bd18b-a505-468b-8de1-ac7f9555dc79","amount":19.000,"currency":"QAR","transactionId":"custom-internal-id","custom1":"test","visaId":null,"status":"new"},"returnCode":200,"errorCode":0,"errorMessage":null,"error":null,"validationErrors":null,"hasError":false,"hasValidationError":false}"#,
        )
        .unwrap();
        let d = env.result_obj.unwrap();
        assert_eq!(d.status_id, 0);
        assert!(
            d.pay_url
                .unwrap()
                .ends_with("/pay/770bd18b-a505-468b-8de1-ac7f9555dc79")
        );
    }
}
