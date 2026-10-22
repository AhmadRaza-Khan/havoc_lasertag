//! SkipCash request and webhook signatures (manual §2.1.1 and §4.2.1): the non-empty fields as
//! `Name=value` joined by commas, in a fixed order, HMAC-SHA256 with the key, base64.

use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;

/// `Name=value,Name=value` for the non-empty values, in the given order.
pub fn data(fields: &[(&str, Option<&str>)]) -> String {
    fields
        .iter()
        .filter_map(|(name, value)| {
            value
                .filter(|v| !v.is_empty())
                .map(|v| format!("{name}={v}"))
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// base64(HMAC-SHA256(key, data)).
pub fn sign(key: &str, data: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("HMAC takes any key length");
    mac.update(data.as_bytes());
    STANDARD.encode(mac.finalize().into_bytes())
}

/// What SkipCash POSTs to our webhook URL. Numbers may arrive as JSON numbers or strings.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookPayload {
    pub payment_id: String,
    #[serde(default, deserialize_with = "text")]
    pub amount: Option<String>,
    #[serde(default, deserialize_with = "text")]
    pub status_id: Option<String>,
    #[serde(default)]
    pub transaction_id: Option<String>,
    #[serde(default)]
    pub custom1: Option<String>,
    #[serde(default)]
    pub visa_id: Option<String>,
}

fn text<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(match Option::<serde_json::Value>::deserialize(d)? {
        Some(serde_json::Value::String(s)) => Some(s),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        _ => None,
    })
}

impl WebhookPayload {
    pub fn signed_data(&self) -> String {
        data(&[
            ("PaymentId", Some(&self.payment_id)),
            ("Amount", self.amount.as_deref()),
            ("StatusId", self.status_id.as_deref()),
            ("TransactionId", self.transaction_id.as_deref()),
            ("Custom1", self.custom1.as_deref()),
            ("VisaId", self.visa_id.as_deref()),
        ])
    }
}

/// Whether the `Authorization` header is SkipCash's signature of the payload with our webhook key.
pub fn verify_webhook(webhook_key: &str, payload: &WebhookPayload, authorization: &str) -> bool {
    let expected = sign(webhook_key, &payload.signed_data());
    let given = authorization.trim();
    expected.len() == given.len()
        && expected
            .bytes()
            .zip(given.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_data_like_the_manual() {
        // Manual §2.1.1: empty fields are left out; the order is fixed.
        assert_eq!(
            data(&[
                ("Uid", Some("01916FC2-411B-4465-A3A8-F7699AD2F757")),
                ("KeyId", Some("key-1")),
                ("Amount", Some("11")),
                ("FirstName", Some("Peter")),
                ("LastName", Some("Green")),
                ("Phone", None),
                ("Email", Some("green@test.sk")),
                ("Street", Some("")),
            ]),
            "Uid=01916FC2-411B-4465-A3A8-F7699AD2F757,KeyId=key-1,Amount=11,FirstName=Peter,LastName=Green,Email=green@test.sk"
        );
    }

    #[test]
    fn signs_with_hmac_sha256_base64() {
        // Known answer (Python: base64(hmac.new(b"secret", b"Amount=11", sha256).digest())).
        assert_eq!(
            sign("secret", "Amount=11"),
            "aoxc6cdFryqfOGFz+NQP028R69eT6W1+RQTj2/nsRhM="
        );
    }

    #[test]
    fn verifies_webhooks() {
        // Manual §4.2.1 example payload (null fields are left out of the signed data).
        let payload: WebhookPayload = serde_json::from_str(
            r#"{"paymentId":"c6409932-9c11-461b-b508-cb094d7db4f6","amount":"11.00","statusId":2,"transactionId":null,"custom1":null,"visaId":"6251217598876165804006"}"#,
        )
        .unwrap();
        assert_eq!(
            payload.signed_data(),
            "PaymentId=c6409932-9c11-461b-b508-cb094d7db4f6,Amount=11.00,StatusId=2,VisaId=6251217598876165804006"
        );
        let good = sign("webhook-key", &payload.signed_data());
        assert!(verify_webhook("webhook-key", &payload, &good));
        assert!(!verify_webhook("other-key", &payload, &good));
        assert!(!verify_webhook("webhook-key", &payload, "nonsense"));
    }
}
