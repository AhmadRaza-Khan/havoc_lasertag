//! Customer emails about paying for a booking.

use chrono::{DateTime, Utc};

use super::{
    model::{PayableBooking, Payment},
    service::Unpaid,
};
use crate::{integrations::email::Email, state::AppState};

pub fn payment_requested(
    state: &AppState,
    to: &str,
    name: &str,
    b: &PayableBooking,
    pay_url: Option<&str>,
) -> Email {
    let due = b.payment_due_at.map(|d| when(state, d)).unwrap_or_default();
    let amount = amount(b);
    let action = match pay_url {
        Some(_) => "Pay securely with the link below or from My bookings in the Havoc app.",
        None => "Complete the payment in the Havoc app.",
    };
    let lines = [
        format!("Hi {name},"),
        format!(
            "Thanks for booking {} on {} (booking #{}).",
            b.product_name,
            when(state, b.start_time),
            b.bookeo_booking_number
        ),
        format!(
            "Please pay {amount} by {due}. If it isn't paid by then, the booking is canceled automatically."
        ),
        action.to_string(),
    ];
    build(
        to,
        format!("Pay for your booking #{} by {due}", b.bookeo_booking_number),
        &lines,
        pay_url.map(|u| ("Pay now", u)),
    )
}

pub fn payment_received(
    state: &AppState,
    to: &str,
    name: &str,
    b: &PayableBooking,
    p: &Payment,
) -> Email {
    let lines = [
        format!("Hi {name},"),
        format!(
            "We received your payment of {} {} for booking #{}: {} on {}.",
            p.amount,
            p.currency,
            b.bookeo_booking_number,
            b.product_name,
            when(state, b.start_time)
        ),
        format!(
            "Payment reference: {}.",
            p.visa_id.as_deref().unwrap_or(&p.merchant_txn_id)
        ),
        "See you at Havoc!".to_string(),
    ];
    build(
        to,
        format!("Payment received for booking #{}", b.bookeo_booking_number),
        &lines,
        None,
    )
}

pub fn canceled_unpaid(
    state: &AppState,
    to: &str,
    name: &str,
    b: &PayableBooking,
    why: Unpaid,
) -> Email {
    let reason = match why {
        Unpaid::TimeRanOut => "it wasn't paid in time",
        Unpaid::LeftPaymentPage => "the payment wasn't completed",
    };
    let lines = [
        format!("Hi {name},"),
        format!(
            "Your booking #{} ({} on {}) was canceled because {reason}. You haven't been charged.",
            b.bookeo_booking_number,
            b.product_name,
            when(state, b.start_time)
        ),
        "You can book again in the Havoc app.".to_string(),
    ];
    build(
        to,
        format!(
            "Booking #{} canceled: not paid in time",
            b.bookeo_booking_number
        ),
        &lines,
        None,
    )
}

fn build(to: &str, subject: String, lines: &[String], button: Option<(&str, &str)>) -> Email {
    let mut text = lines.join("\n\n");
    let mut html: String = lines
        .iter()
        .map(|l| format!("<p>{}</p>", escape(l)))
        .collect();
    if let Some((label, url)) = button {
        text.push_str(&format!("\n\n{label}: {url}"));
        html.push_str(&format!(
            r#"<p><a href="{}" style="display:inline-block;padding:10px 18px;background:#d71920;color:#fff;border-radius:6px;text-decoration:none;font-weight:600">{}</a></p>"#,
            escape(url),
            escape(label)
        ));
    }
    text.push_str("\n\nHavoc Tactical Laser Tag");
    html.push_str("<p style=\"color:#666\">Havoc Tactical Laser Tag</p>");
    Email {
        to: to.to_string(),
        subject,
        text,
        html: format!(
            "<div style=\"font-family:Arial,sans-serif;font-size:15px;line-height:1.5\">{html}</div>"
        ),
    }
}

/// e.g. "Sat 10 Oct 2026, 14:00" in the venue's timezone.
fn when(state: &AppState, t: DateTime<Utc>) -> String {
    t.with_timezone(&state.config.bookeo.timezone)
        .format("%a %-d %b %Y, %H:%M")
        .to_string()
}

fn amount(b: &PayableBooking) -> String {
    match (&b.price_amount, &b.price_currency) {
        (Some(a), Some(c)) => format!("{a} {c}"),
        _ => "the booking price".into(),
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_html_but_not_text() {
        let e = build(
            "a@b.c",
            "s".into(),
            &["Hi <b>Tom & Jerry</b>".into()],
            Some(("Pay", "https://x/?a=1&b=2")),
        );
        assert!(e.html.contains("Hi &lt;b&gt;Tom &amp; Jerry&lt;/b&gt;"));
        assert!(e.html.contains(r#"href="https://x/?a=1&amp;b=2""#));
        assert!(e.text.contains("Hi <b>Tom & Jerry</b>"));
        assert!(e.text.contains("Pay: https://x/?a=1&b=2"));
    }
}
