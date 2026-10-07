use std::time::{Duration, Instant};

use chrono::NaiveDate;
use reqwest::{Method, RequestBuilder, StatusCode, header::AUTHORIZATION};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;

use crate::{config::QnbConfig, error::AppError};

/// QNB sessions last 15 minutes; renew a little early.
const TOKEN_TTL: Duration = Duration::from_secs(14 * 60);
const API_VERSION: &str = "1.00";
/// Opening the link counts as a click; allow reopening it (app restarts, retries).
const MAX_CLICKS: &str = "10";

#[derive(Debug, thiserror::Error)]
pub enum QnbError {
    /// QNB refused the request (`errorCode` in the body).
    #[error("{0}")]
    Rejected(String),
    /// Network failure, outage, bad credentials, unexpected response.
    #[error("{0}")]
    Unavailable(String),
}

impl From<QnbError> for AppError {
    fn from(e: QnbError) -> Self {
        AppError::BadGateway(format!("qnb: {e}"))
    }
}

type QnbResult<T> = Result<T, QnbError>;

pub struct PayLinkRequest<'a> {
    /// Our unique reference (max 20 characters).
    pub merchant_txn_id: &'a str,
    /// Decimal string with 2 places, e.g. "400.00".
    pub amount: &'a str,
    pub currency: &'a str,
    /// Shown in the QNB merchant console next to the transaction.
    pub reference: &'a str,
    /// Last day the link can be used.
    pub due_date: NaiveDate,
    /// Arabic payment page instead of English.
    pub arabic: bool,
}

#[derive(Debug)]
pub struct PayLink {
    pub ipg_txn_id: String,
    pub url: String,
}

/// What QNB says about a payment link's transaction.
#[derive(Debug, PartialEq)]
pub enum LinkStatus {
    /// Paid. The amount and currency QNB charged.
    Accepted { amount: String, currency: String },
    /// The latest attempt failed (e.g. card declined); the link may still be retried.
    Rejected(String),
    /// Not paid yet.
    Pending,
}

pub struct QnbClient {
    http: reqwest::Client,
    config: QnbConfig,
    token: Mutex<Option<(Instant, String)>>,
}

impl QnbClient {
    pub fn new(config: &QnbConfig) -> Result<Self, reqwest::Error> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(10))
                .build()?,
            config: config.clone(),
            token: Mutex::new(None),
        })
    }

    /// Creates a hosted payment link for a sale (spec 5.23 Pay By Link).
    pub async fn create_pay_link(&self, req: &PayLinkRequest<'_>) -> QnbResult<PayLink> {
        let c = &self.config;
        let body = PayLinkBody {
            merchant_id: &c.merchant_id,
            merchant_pwd: &c.merchant_pwd,
            merchant_key: &c.merchant_key,
            merchant_txn_id: req.merchant_txn_id,
            txn_amt: req.amount,
            curr_code: req.currency,
            merchant_var1: req.reference,
            max_allowed_clicks: MAX_CLICKS,
            due_date: req.due_date.format("%Y-%m-%d").to_string(),
            version: API_VERSION,
            lang: if req.arabic { "ara" } else { "eng" },
            pre_auth: "N",
        };
        let res: PayLinkResponse = self
            .call(Method::POST, "/ipg/rest/v1/payments/paybylink", Some(&body))
            .await?;
        if let Some(e) = error_of(&res.error_code, &res.error_desc) {
            return Err(QnbError::Rejected(e));
        }
        let ipg_txn_id = res
            .ipg_txn_id
            .filter(|id| !id.is_empty())
            .ok_or_else(|| QnbError::Unavailable("pay link response without ipgTxnId".into()))?;
        let url = res
            .links
            .into_iter()
            .find(|l| l.rel == "payLink")
            .map(|l| l.link)
            .ok_or_else(|| QnbError::Unavailable("pay link response without a payLink".into()))?;
        Ok(PayLink { ipg_txn_id, url })
    }

    /// Current state of a link's transaction (spec 5.21 Single Transaction Inquiry).
    pub async fn link_status(&self, ipg_txn_id: &str) -> QnbResult<LinkStatus> {
        let path = format!("/ipg/rest/v1/payments/{}", path_segment(ipg_txn_id)?);
        let res: Inquiry = self.call::<(), _>(Method::GET, &path, None).await?;
        Ok(res.status())
    }

    /// Disables a link so it can no longer be paid.
    pub async fn cancel_pay_link(&self, ipg_txn_id: &str) -> QnbResult<()> {
        let path = format!(
            "/ipg/rest/v1/payByLink/{}/cancel",
            path_segment(ipg_txn_id)?
        );
        let res: ErrorFields = self.call::<(), _>(Method::GET, &path, None).await?;
        match error_of(&res.error_code, &res.error_desc) {
            Some(e) => Err(QnbError::Rejected(e)),
            None => Ok(()),
        }
    }

    // --- internals ---

    /// Sends an authenticated request, renewing the session once if QNB rejects the token.
    async fn call<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> QnbResult<T> {
        let url = format!("{}{path}", self.config.base_url);
        for attempt in 0..2 {
            let token = self.token(attempt > 0).await?;
            let mut req: RequestBuilder = self
                .http
                .request(method.clone(), &url)
                .header(AUTHORIZATION, bearer(&token));
            if let Some(b) = body {
                req = req.json(b);
            }
            let res = req
                .send()
                .await
                .map_err(|e| QnbError::Unavailable(format!("request failed: {e}")))?;
            let status = res.status();
            if status == StatusCode::UNAUTHORIZED && attempt == 0 {
                continue;
            }
            let text = res
                .text()
                .await
                .map_err(|e| QnbError::Unavailable(format!("reading response: {e}")))?;
            if status.is_server_error() || status == StatusCode::UNAUTHORIZED {
                return Err(QnbError::Unavailable(format!(
                    "{status}: {}",
                    snippet(&text)
                )));
            }
            // QNB reports business errors as JSON with errorCode, sometimes with a 4xx status.
            return serde_json::from_str(&text).map_err(|e| {
                if status.is_client_error() {
                    QnbError::Rejected(format!("{status}: {}", snippet(&text)))
                } else {
                    QnbError::Unavailable(format!("unexpected response ({e}): {}", snippet(&text)))
                }
            });
        }
        Err(QnbError::Unavailable("authentication failed".into()))
    }

    /// The session JWT (spec 5.2), cached for [`TOKEN_TTL`]. `renew` forces a new one.
    async fn token(&self, renew: bool) -> QnbResult<String> {
        let mut cached = self.token.lock().await;
        if !renew
            && let Some((at, token)) = cached.as_ref()
            && at.elapsed() < TOKEN_TTL
        {
            return Ok(token.clone());
        }

        let c = &self.config;
        let res = self
            .http
            .post(format!("{}/ipg/rest/v1/access/token", c.base_url))
            .json(&TokenRequest {
                user_id: &c.user_id,
                merchant_id: &c.merchant_id,
                merchant_pwd: &c.merchant_pwd,
            })
            .send()
            .await
            .map_err(|e| QnbError::Unavailable(format!("token request failed: {e}")))?;
        let status = res.status();
        let token = res
            .headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.trim_start_matches("Bearer ").trim().to_string())
            .filter(|t| !t.is_empty());
        let Some(token) = token else {
            let body = res.text().await.unwrap_or_default();
            return Err(QnbError::Unavailable(format!(
                "no session token ({status}): {}",
                snippet(&body)
            )));
        };
        *cached = Some((Instant::now(), token.clone()));
        Ok(token)
    }
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

/// Transaction ids are digits; refuse anything that could change the URL path.
fn path_segment(id: &str) -> QnbResult<&str> {
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(id)
    } else {
        Err(QnbError::Unavailable(format!(
            "invalid transaction id {id:?}"
        )))
    }
}

fn snippet(text: &str) -> String {
    text.chars().take(300).collect()
}

fn error_of(code: &Option<String>, desc: &Option<String>) -> Option<String> {
    let code = code.as_deref().map(str::trim).filter(|c| !c.is_empty())?;
    Some(
        match desc.as_deref().map(str::trim).filter(|d| !d.is_empty()) {
            // QNB descriptions usually repeat the code: "ERR_00_53 - Duplicate transaction ..."
            Some(d) if d.starts_with(code) => d.to_string(),
            Some(d) => format!("{code} - {d}"),
            None => code.to_string(),
        },
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenRequest<'a> {
    user_id: &'a str,
    merchant_id: &'a str,
    merchant_pwd: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PayLinkBody<'a> {
    merchant_id: &'a str,
    merchant_pwd: &'a str,
    merchant_key: &'a str,
    merchant_txn_id: &'a str,
    txn_amt: &'a str,
    curr_code: &'a str,
    merchant_var1: &'a str,
    max_allowed_clicks: &'a str,
    due_date: String,
    version: &'a str,
    lang: &'a str,
    pre_auth: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PayLinkResponse {
    ipg_txn_id: Option<String>,
    #[serde(default)]
    links: Vec<Link>,
    error_code: Option<String>,
    error_desc: Option<String>,
}

#[derive(Deserialize)]
struct Link {
    link: String,
    rel: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorFields {
    error_code: Option<String>,
    error_desc: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Inquiry {
    txn_status: Option<String>,
    txn_status_description: Option<String>,
    fail_reason: Option<String>,
    txn_amt: Option<String>,
    curr_code: Option<String>,
    error_code: Option<String>,
    error_desc: Option<String>,
}

impl Inquiry {
    fn status(self) -> LinkStatus {
        // Spec values are "ACCEPTED" / "REJECTED"; samples also show "ACCEPTED.".
        let status = self
            .txn_status
            .as_deref()
            .unwrap_or_default()
            .trim()
            .trim_end_matches('.')
            .to_ascii_uppercase();
        match status.as_str() {
            "ACCEPTED" => LinkStatus::Accepted {
                amount: self.txn_amt.unwrap_or_default(),
                currency: self.curr_code.unwrap_or_default(),
            },
            "REJECTED" => LinkStatus::Rejected(
                [
                    self.fail_reason,
                    self.txn_status_description,
                    error_of(&self.error_code, &self.error_desc),
                ]
                .into_iter()
                .flatten()
                .find(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Payment was declined".into()),
            ),
            // No attempt yet (QNB may answer with an error such as "not found"), or in progress.
            _ => LinkStatus::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inquiry(json: &str) -> LinkStatus {
        serde_json::from_str::<Inquiry>(json).unwrap().status()
    }

    #[test]
    fn reads_inquiry_status() {
        assert_eq!(
            inquiry(r#"{"txnStatus":"ACCEPTED.","txnAmt":"400.00","currCode":"QAR"}"#),
            LinkStatus::Accepted {
                amount: "400.00".into(),
                currency: "QAR".into()
            }
        );
        assert_eq!(
            inquiry(r#"{"txnStatus":"REJECTED","failReason":"Insufficient funds"}"#),
            LinkStatus::Rejected("Insufficient funds".into())
        );
        assert_eq!(
            inquiry(
                r#"{"txnStatus":"REJECTED","failReason":"","errorCode":"ERR_1","errorDesc":"Declined"}"#
            ),
            LinkStatus::Rejected("ERR_1 - Declined".into())
        );
        assert_eq!(
            inquiry(r#"{"errorCode":"ERR_04_01","errorDesc":"Not found"}"#),
            LinkStatus::Pending
        );
        assert_eq!(inquiry(r#"{"txnStatus":""}"#), LinkStatus::Pending);
    }

    #[test]
    fn rejects_unsafe_ids() {
        assert!(path_segment("0017000010081361743111867896").is_ok());
        assert!(path_segment("../x").is_err());
        assert!(path_segment("").is_err());
    }
}
