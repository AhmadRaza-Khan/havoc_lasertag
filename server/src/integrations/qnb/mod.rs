//! QNB Internet Payment Gateway (ibSuite Payments REST API, spec QAT-QNB-01-IP r6.1).
//!
//! Only the hosted flow is used: "Pay by Link" creates a QNB payment page that the app opens
//! in a WebView, so card details never reach our app or server. Whether a link was paid is
//! confirmed server-side with the transaction inquiry — never from the WebView's URL.

mod client;

pub use client::{LinkStatus, PayLink, PayLinkRequest, QnbClient, QnbError};
