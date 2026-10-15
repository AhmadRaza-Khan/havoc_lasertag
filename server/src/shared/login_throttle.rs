//! Brute-force protection for the login endpoints (in memory, per process).
//!
//! Two limits, both over a sliding window:
//! - per client IP + account: stops one attacker hammering one account
//! - per account from anywhere: stops distributed guessing against one account
//!
//! A successful login clears that client's counter.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::http::HeaderMap;

use crate::error::{AppError, AppResult};

const WINDOW: Duration = Duration::from_secs(15 * 60);
const MAX_PER_CLIENT: u32 = 5;
const MAX_PER_ACCOUNT: u32 = 20;
/// Drop expired entries once the map grows past this.
const PRUNE_AT: usize = 10_000;

#[derive(Default)]
pub struct LoginThrottle {
    failures: Mutex<HashMap<String, (u32, Instant)>>,
}

/// Identifies a login attempt: which login form, which account, from where.
pub struct Attempt {
    client_key: String,
    account_key: String,
}

impl Attempt {
    /// `realm` separates admin and player logins; `account` should be normalized.
    pub fn new(realm: &str, account: &str, headers: &HeaderMap) -> Self {
        let ip = client_ip(headers);
        Self {
            client_key: format!("{realm}|{ip}|{account}"),
            account_key: format!("{realm}|*|{account}"),
        }
    }
}

impl Attempt {
    /// Counts per client network only (both keys are the IP), e.g. for limiting sign-ups.
    pub fn per_client(realm: &str, headers: &HeaderMap) -> Self {
        let ip = client_ip(headers);
        Self {
            client_key: format!("{realm}|{ip}"),
            account_key: format!("{realm}|{ip}|all"),
        }
    }
}

impl LoginThrottle {
    /// Rejects with 429 if this client or account is locked out.
    pub fn check(&self, attempt: &Attempt) -> AppResult<()> {
        let failures = self.lock();
        let blocked = |key: &str, max: u32| {
            failures
                .get(key)
                .filter(|(count, since)| *count >= max && since.elapsed() < WINDOW)
                .map(|(_, since)| WINDOW.saturating_sub(since.elapsed()))
        };
        if let Some(wait) = blocked(&attempt.client_key, MAX_PER_CLIENT)
            .or_else(|| blocked(&attempt.account_key, MAX_PER_ACCOUNT))
        {
            let minutes = wait.as_secs().div_ceil(60).max(1);
            return Err(AppError::TooManyRequests(format!(
                "Too many failed sign-in attempts. Try again in {minutes} minute(s)."
            )));
        }
        Ok(())
    }

    pub fn record_failure(&self, attempt: &Attempt) {
        let mut failures = self.lock();
        if failures.len() > PRUNE_AT {
            failures.retain(|_, (_, since)| since.elapsed() < WINDOW);
        }
        for key in [&attempt.client_key, &attempt.account_key] {
            let entry = failures.entry(key.clone()).or_insert((0, Instant::now()));
            if entry.1.elapsed() >= WINDOW {
                *entry = (0, Instant::now());
            }
            entry.0 += 1;
        }
    }

    pub fn record_success(&self, attempt: &Attempt) {
        self.lock().remove(&attempt.client_key);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, (u32, Instant)>> {
        self.failures.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The API only listens on 127.0.0.1 behind nginx, which sets `X-Real-IP` to the real client
/// address (overwriting anything the client sent). Without it (local dev), all requests share
/// one bucket.
fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("local")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(ip: &str, account: &str) -> Attempt {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", ip.parse().unwrap());
        Attempt::new("player", account, &headers)
    }

    #[test]
    fn locks_client_after_repeated_failures() {
        let t = LoginThrottle::default();
        let a = attempt("1.1.1.1", "bob");
        for _ in 0..MAX_PER_CLIENT {
            assert!(t.check(&a).is_ok());
            t.record_failure(&a);
        }
        assert!(matches!(t.check(&a), Err(AppError::TooManyRequests(_))));
        // A different client for the same account is not yet blocked
        assert!(t.check(&attempt("2.2.2.2", "bob")).is_ok());
        // ...nor the same client for another account
        assert!(t.check(&attempt("1.1.1.1", "alice")).is_ok());
    }

    #[test]
    fn locks_account_under_distributed_guessing() {
        let t = LoginThrottle::default();
        for i in 0..MAX_PER_ACCOUNT {
            t.record_failure(&attempt(&format!("10.0.0.{i}"), "bob"));
        }
        assert!(t.check(&attempt("9.9.9.9", "bob")).is_err());
    }

    #[test]
    fn success_clears_client_counter() {
        let t = LoginThrottle::default();
        let a = attempt("1.1.1.1", "bob");
        for _ in 0..MAX_PER_CLIENT - 1 {
            t.record_failure(&a);
        }
        t.record_success(&a);
        t.record_failure(&a);
        assert!(t.check(&a).is_ok());
    }
}
