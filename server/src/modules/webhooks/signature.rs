//! Checks that a webhook call comes from Bookeo (https://www.bookeo.com/api/webhooks/,
//! "Message authentication"): `X-Bookeo-Signature` is the hex HMAC-SHA256, keyed with our
//! application's secret key, of timestamp + message id + webhook URL + body, and
//! `X-Bookeo-Timestamp` (ms since the epoch) must be recent.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// How far the call's timestamp may be from our clock (Bookeo suggests ~120 s).
pub const MAX_SKEW_MS: i64 = 5 * 60 * 1000;

#[derive(Debug, PartialEq, Eq)]
pub enum Invalid {
    BadTimestamp,
    StaleTimestamp,
    BadSignature,
}

pub struct Signed<'a> {
    pub timestamp: &'a str,
    pub message_id: &'a str,
    pub signature: &'a str,
    pub body: &'a [u8],
}

/// `url` is the webhook URL exactly as registered with Bookeo.
pub fn verify(secret: &str, url: &str, msg: &Signed, now_ms: i64) -> Result<(), Invalid> {
    let ts: i64 = msg
        .timestamp
        .trim()
        .parse()
        .map_err(|_| Invalid::BadTimestamp)?;
    if (now_ms - ts).abs() > MAX_SKEW_MS {
        return Err(Invalid::StaleTimestamp);
    }
    let expected = hex::decode(msg.signature.trim()).map_err(|_| Invalid::BadSignature)?;

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC takes any key length");
    mac.update(msg.timestamp.as_bytes());
    mac.update(msg.message_id.as_bytes());
    mac.update(url.as_bytes());
    mac.update(msg.body);
    // Constant-time comparison.
    mac.verify_slice(&expected)
        .map_err(|_| Invalid::BadSignature)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The worked example from Bookeo's webhook documentation.
    const SECRET: &str = "iWQlbsuksGUqStFPk46WVjpGO7vVQoeO";
    const URL: &str = "https://www.mywebsite.com/webhooktest?id=1234";
    const TS: &str = "1683025420401";
    const ID: &str = "dvpwVQI0W7Pe187dc203154";
    const BODY: &str = r#"{"itemId":"2856MUMPA187DC203130","timestamp":"2023-05-02T04:01:00-07:00","item":{"credit":{"amount":"0","currency":"USD"},"acceptSmsReminders":true,"numBookings":0,"numCancelations":0,"numNoShows":0,"member":false,"id":"2856MUMPA187DC203130","firstName":"John","lastName":"Smith","streetAddress":{"countryCode":"US"},"creationTime":"2023-05-02T04:01:00-07:00","gender":"unknown"}}"#;
    const SIG: &str = "a3cec455a9462fc524b02eeac7a26af743d512763271ab170f957aeafd6a636e";
    const NOW: i64 = 1683025420401;

    fn msg<'a>(signature: &'a str, body: &'a str) -> Signed<'a> {
        Signed {
            timestamp: TS,
            message_id: ID,
            signature,
            body: body.as_bytes(),
        }
    }

    #[test]
    fn accepts_bookeos_example() {
        assert_eq!(verify(SECRET, URL, &msg(SIG, BODY), NOW), Ok(()));
        assert_eq!(
            verify(SECRET, URL, &msg(SIG, BODY), NOW + 60_000),
            Ok(()),
            "a minute late is fine"
        );
    }

    #[test]
    fn rejects_tampering_and_wrong_keys() {
        let tampered = BODY.replace("Smith", "Smyth");
        assert_eq!(
            verify(SECRET, URL, &msg(SIG, &tampered), NOW),
            Err(Invalid::BadSignature)
        );
        assert_eq!(
            verify("other-secret", URL, &msg(SIG, BODY), NOW),
            Err(Invalid::BadSignature)
        );
        assert_eq!(
            verify(
                SECRET,
                "https://www.mywebsite.com/other",
                &msg(SIG, BODY),
                NOW
            ),
            Err(Invalid::BadSignature),
            "signed for another URL"
        );
        assert_eq!(
            verify(SECRET, URL, &msg("not-hex", BODY), NOW),
            Err(Invalid::BadSignature)
        );
    }

    #[test]
    fn rejects_old_or_garbled_timestamps() {
        assert_eq!(
            verify(SECRET, URL, &msg(SIG, BODY), NOW + MAX_SKEW_MS + 1),
            Err(Invalid::StaleTimestamp)
        );
        assert_eq!(
            verify(SECRET, URL, &msg(SIG, BODY), NOW - MAX_SKEW_MS - 1),
            Err(Invalid::StaleTimestamp)
        );
        let bad = Signed {
            timestamp: "yesterday",
            ..msg(SIG, BODY)
        };
        assert_eq!(verify(SECRET, URL, &bad, NOW), Err(Invalid::BadTimestamp));
    }
}
