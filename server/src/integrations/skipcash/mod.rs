//! SkipCash online card payments (Card Checkout API, SkipCash Integration Manual v1.4).
//!
//! The hosted flow: we create a payment (signed with our private key) and get SkipCash's
//! `payUrl`, which the app opens in a WebView, so card details never reach our app or server.
//! Whether a payment was paid is confirmed server-side (`GET /api/v1/payments/{id}`) or by
//! SkipCash's signed webhook — never from the WebView's URL.

mod client;
mod signature;

pub use client::{
    CreatedPayment, NewPayment, PaymentState, SkipCashClient, SkipCashConfig, SkipCashError,
};
pub use signature::{WebhookPayload, verify_webhook};
