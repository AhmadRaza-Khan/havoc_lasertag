use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, SecondsFormat, TimeZone};
use reqwest::{Method, RequestBuilder, Response, StatusCode, header::LOCATION};
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::RwLock;

use super::types::{Booking, Customer, NewBooking, NewCustomer, Page, Product, Slot};
use crate::{config::BookeoConfig, error::AppError};

/// Bookeo recommends caching product settings for 10-15 minutes.
const PRODUCTS_TTL: Duration = Duration::from_secs(10 * 60);
/// Safety net against a pagination loop.
const MAX_PAGES: i32 = 50;

#[derive(Debug, thiserror::Error)]
pub enum BookeoError {
    /// Bookeo refused the request (e.g. slot full, invalid event). Safe to show to the user.
    #[error("{0}")]
    Rejected(String),
    /// Network failure, Bookeo outage, throttling, bad credentials, unexpected response.
    #[error("{0}")]
    Unavailable(String),
}

impl From<BookeoError> for AppError {
    fn from(e: BookeoError) -> Self {
        match e {
            BookeoError::Rejected(m) => AppError::Unprocessable(m),
            BookeoError::Unavailable(m) => AppError::BadGateway(format!("bookeo: {m}")),
        }
    }
}

type BookeoResult<T> = Result<T, BookeoError>;

/// Per-request options for booking writes.
#[derive(Debug, Clone, Copy)]
pub struct RequestOptions {
    /// Email/SMS the business's staff about the change.
    pub notify_users: bool,
    /// Email the customer about the change.
    pub notify_customer: bool,
    /// Act as a manager (`mode=backend`): relaxes customer-facing limits such as how far in
    /// advance a booking must be made. Use for admin-panel actions only.
    pub backend: bool,
}

impl RequestOptions {
    fn query(self) -> Vec<(&'static str, &'static str)> {
        let flag = |b: bool| if b { "true" } else { "false" };
        let mut q = vec![
            ("notifyUsers", flag(self.notify_users)),
            ("notifyCustomer", flag(self.notify_customer)),
        ];
        if self.backend {
            q.push(("mode", "backend"));
        }
        q
    }
}

pub struct BookeoClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    secret_key: String,
    products_cache: RwLock<Option<(Instant, Arc<Vec<Product>>)>>,
}

impl BookeoClient {
    pub fn new(config: &BookeoConfig) -> Result<Self, reqwest::Error> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(10))
                .build()?,
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            secret_key: config.secret_key.clone(),
            products_cache: RwLock::new(None),
        })
    }

    /// All products (games), cached for [`PRODUCTS_TTL`].
    pub async fn products(&self) -> BookeoResult<Arc<Vec<Product>>> {
        if let Some((fetched_at, products)) = self.products_cache.read().await.as_ref()
            && fetched_at.elapsed() < PRODUCTS_TTL
        {
            return Ok(products.clone());
        }

        let products = Arc::new(
            self.get_all_pages::<Product>("/settings/products", &[("itemsPerPage", "100".into())])
                .await?,
        );
        *self.products_cache.write().await = Some((Instant::now(), products.clone()));
        Ok(products)
    }

    /// Slots for one product between `start` and `end` (max 31 days apart), including full ones.
    pub async fn slots<Tz: TimeZone>(
        &self,
        product_id: &str,
        start: &DateTime<Tz>,
        end: &DateTime<Tz>,
    ) -> BookeoResult<Vec<Slot>>
    where
        Tz::Offset: std::fmt::Display,
    {
        self.get_all_pages(
            "/availability/slots",
            &[
                ("productId", product_id.to_string()),
                ("startTime", bookeo_time(start)),
                ("endTime", bookeo_time(end)),
                ("itemsPerPage", "300".into()),
            ],
        )
        .await
    }

    /// Creates a customer and returns its Bookeo id.
    pub async fn create_customer(&self, customer: &NewCustomer) -> BookeoResult<String> {
        let res = self
            .send(self.request(Method::POST, "/customers").json(customer))
            .await?;
        let created: Customer = self.created_resource(res).await?;
        Ok(created.id)
    }

    pub async fn create_booking(
        &self,
        booking: &NewBooking,
        opts: RequestOptions,
    ) -> BookeoResult<Booking> {
        let req = self
            .request(Method::POST, "/bookings")
            .query(&opts.query())
            .json(booking);
        let res = self.send(req).await?;
        self.created_resource(res).await
    }

    /// Replaces a booking's slot/participants. Bookeo answers 200, possibly without a body.
    pub async fn update_booking(
        &self,
        booking_number: &str,
        booking: &NewBooking,
        opts: RequestOptions,
    ) -> BookeoResult<Booking> {
        let req = self
            .request(Method::PUT, &format!("/bookings/{booking_number}"))
            .query(&opts.query())
            .json(booking);
        let res = self.send(req).await?;
        let body = res
            .text()
            .await
            .map_err(|e| BookeoError::Unavailable(format!("reading response: {e}")))?;
        match serde_json::from_str(&body) {
            Ok(b) => Ok(b),
            Err(_) => self.booking(booking_number).await,
        }
    }

    /// Cancels a booking. It stays in Bookeo's records but frees its seats.
    pub async fn cancel_booking(
        &self,
        booking_number: &str,
        opts: RequestOptions,
    ) -> BookeoResult<()> {
        let mut query = opts.query();
        query.push(("trackInCustomerHistory", "true"));
        self.send(
            self.request(Method::DELETE, &format!("/bookings/{booking_number}"))
                .query(&query),
        )
        .await?;
        Ok(())
    }

    pub async fn booking(&self, booking_number: &str) -> BookeoResult<Booking> {
        let res = self
            .send(self.request(Method::GET, &format!("/bookings/{booking_number}")))
            .await?;
        json_body(res).await
    }

    // --- internals ---

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.http
            .request(method, format!("{}{path}", self.base_url))
            .header("X-Bookeo-apiKey", &self.api_key)
            .header("X-Bookeo-secretKey", &self.secret_key)
    }

    /// Sends a request and turns non-2xx responses into [`BookeoError`]s.
    async fn send(&self, req: RequestBuilder) -> BookeoResult<Response> {
        let res = req
            .send()
            .await
            .map_err(|e| BookeoError::Unavailable(format!("request failed: {e}")))?;

        let status = res.status();
        if status.is_success() {
            return Ok(res);
        }

        let body = res.text().await.unwrap_or_default();
        let message = error_message(&body);
        Err(match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                BookeoError::Unavailable(format!("auth failed ({status}): {message}"))
            }
            StatusCode::TOO_MANY_REQUESTS => BookeoError::Unavailable("throttled (429)".into()),
            s if s.is_client_error() => BookeoError::Rejected(message),
            _ => BookeoError::Unavailable(format!("{status}: {message}")),
        })
    }

    /// Bookeo answers creates with 201 and the resource in the body; if the body is empty,
    /// fall back to fetching the `Location` header.
    async fn created_resource<T: DeserializeOwned>(&self, res: Response) -> BookeoResult<T> {
        let location = res
            .headers()
            .get(LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let body = res
            .text()
            .await
            .map_err(|e| BookeoError::Unavailable(format!("reading response: {e}")))?;

        if let Ok(parsed) = serde_json::from_str(&body) {
            return Ok(parsed);
        }
        let location = location.ok_or_else(|| {
            BookeoError::Unavailable("create response had no body and no Location header".into())
        })?;
        // Location is an absolute URL to the new resource
        let res = self
            .send(
                self.http
                    .get(&location)
                    .header("X-Bookeo-apiKey", &self.api_key)
                    .header("X-Bookeo-secretKey", &self.secret_key),
            )
            .await?;
        json_body(res).await
    }

    /// GETs a paginated list endpoint and returns the items of every page.
    async fn get_all_pages<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> BookeoResult<Vec<T>> {
        let first: Page<T> = json_body(
            self.send(self.request(Method::GET, path).query(query))
                .await?,
        )
        .await?;
        let mut items = first.data;

        let Some(info) = first.info else {
            return Ok(items);
        };
        let total_pages = info.total_pages.unwrap_or(1).min(MAX_PAGES);
        let mut token = info.page_navigation_token;
        let mut page = info.current_page.unwrap_or(1);

        while page < total_pages {
            let Some(t) = token.clone() else { break };
            page += 1;
            let next: Page<T> = json_body(
                self.send(self.request(Method::GET, path).query(&PageQuery {
                    page_navigation_token: &t,
                    page_number: page,
                }))
                .await?,
            )
            .await?;
            items.extend(next.data);
            token = next.info.and_then(|i| i.page_navigation_token).or(Some(t));
        }

        Ok(items)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageQuery<'a> {
    page_navigation_token: &'a str,
    page_number: i32,
}

async fn json_body<T: DeserializeOwned>(res: Response) -> BookeoResult<T> {
    let body = res
        .text()
        .await
        .map_err(|e| BookeoError::Unavailable(format!("reading response: {e}")))?;
    serde_json::from_str(&body).map_err(|e| {
        BookeoError::Unavailable(format!(
            "unexpected response ({e}): {}",
            body.chars().take(300).collect::<String>()
        ))
    })
}

/// RFC 3339 with whole seconds: Bookeo answers 404 if the timestamp has fractional seconds.
fn bookeo_time<Tz: TimeZone>(t: &DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    t.to_rfc3339_opts(SecondsFormat::Secs, false)
}

/// Bookeo's error bodies aren't documented; prefer a `message` field, else the raw text.
fn error_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_owned))
        .unwrap_or_else(|| {
            let text = body.trim();
            if text.is_empty() {
                "Request rejected by booking system".into()
            } else {
                text.chars().take(300).collect()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_times_without_fractional_seconds() {
        let t: DateTime<chrono::FixedOffset> =
            "2026-09-26T11:10:00.123456789+03:00".parse().unwrap();
        assert_eq!(bookeo_time(&t), "2026-09-26T11:10:00+03:00");
    }

    #[test]
    fn error_message_prefers_json_message() {
        assert_eq!(
            error_message(r#"{"message":"Not enough seats"}"#),
            "Not enough seats"
        );
        assert_eq!(error_message("plain failure"), "plain failure");
        assert_eq!(error_message(""), "Request rejected by booking system");
    }
}
