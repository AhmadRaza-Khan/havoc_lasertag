//! Resolves the `from`/`to` date query into a concrete time window in the business timezone.

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::error::{AppError, AppResult};

/// Bookeo's `/availability/slots` accepts at most 31 days per call.
pub const MAX_RANGE_DAYS: i64 = 31;

#[derive(Debug, PartialEq)]
pub struct SlotWindow {
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// Start of the search: start of `from`, or `now` if that is later (past slots can't be booked).
    pub start: DateTime<Tz>,
    /// End of the `to` day.
    pub end: DateTime<Tz>,
}

/// - no dates → today
/// - `from` only → that single day; `to` only → today..=to
/// - both dates are inclusive, in the business timezone
pub fn resolve(
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    tz: Tz,
    now: DateTime<Utc>,
) -> AppResult<SlotWindow> {
    let now = now.with_timezone(&tz);
    let today = now.date_naive();

    let (from, to) = match (from, to) {
        (None, None) => (today, today),
        (Some(f), None) => (f, f),
        (None, Some(t)) => (today, t),
        (Some(f), Some(t)) => (f, t),
    };

    if to < from {
        return Err(AppError::BadRequest(
            "`to` must be on or after `from`".into(),
        ));
    }
    if (to - from).num_days() + 1 > MAX_RANGE_DAYS {
        return Err(AppError::BadRequest(format!(
            "Date range can be at most {MAX_RANGE_DAYS} days"
        )));
    }

    let day_start = local(tz, from, NaiveTime::MIN);
    let end = local(tz, to, NaiveTime::MIN) + Duration::days(1) - Duration::seconds(1);
    if end <= now {
        return Err(AppError::BadRequest("Date range is in the past".into()));
    }

    Ok(SlotWindow {
        from,
        to,
        start: day_start.max(now),
        end,
    })
}

fn local(tz: Tz, date: NaiveDate, time: NaiveTime) -> DateTime<Tz> {
    // earliest() handles DST gaps/overlaps; Asia/Qatar has none, but other zones might.
    tz.from_local_datetime(&date.and_time(time))
        .earliest()
        .unwrap_or_else(|| tz.from_utc_datetime(&date.and_time(time)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TZ: Tz = chrono_tz::Asia::Qatar;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    /// 2026-09-26 10:30 in Qatar (07:30 UTC)
    fn now() -> DateTime<Utc> {
        "2026-09-26T07:30:00Z".parse().unwrap()
    }

    #[test]
    fn defaults_to_rest_of_today() {
        let w = resolve(None, None, TZ, now()).unwrap();
        assert_eq!((w.from, w.to), (d("2026-09-26"), d("2026-09-26")));
        assert_eq!(w.start.to_rfc3339(), "2026-09-26T10:30:00+03:00");
        assert_eq!(w.end.to_rfc3339(), "2026-09-26T23:59:59+03:00");
    }

    #[test]
    fn range_is_inclusive_in_local_time() {
        let w = resolve(Some(d("2026-09-27")), Some(d("2026-09-28")), TZ, now()).unwrap();
        assert_eq!(w.start.to_rfc3339(), "2026-09-27T00:00:00+03:00");
        assert_eq!(w.end.to_rfc3339(), "2026-09-28T23:59:59+03:00");
    }

    #[test]
    fn range_starting_today_starts_now() {
        let w = resolve(Some(d("2026-09-26")), Some(d("2026-09-28")), TZ, now()).unwrap();
        assert_eq!(w.start.to_rfc3339(), "2026-09-26T10:30:00+03:00");
    }

    #[test]
    fn single_bound() {
        let w = resolve(Some(d("2026-10-01")), None, TZ, now()).unwrap();
        assert_eq!((w.from, w.to), (d("2026-10-01"), d("2026-10-01")));
        let w = resolve(None, Some(d("2026-09-28")), TZ, now()).unwrap();
        assert_eq!((w.from, w.to), (d("2026-09-26"), d("2026-09-28")));
    }

    #[test]
    fn rejects_bad_ranges() {
        assert!(resolve(Some(d("2026-09-28")), Some(d("2026-09-27")), TZ, now()).is_err());
        assert!(resolve(Some(d("2026-09-26")), Some(d("2026-10-27")), TZ, now()).is_err());
        assert!(resolve(Some(d("2026-09-20")), Some(d("2026-09-25")), TZ, now()).is_err());
        // exactly 31 days is fine
        assert!(resolve(Some(d("2026-09-26")), Some(d("2026-10-26")), TZ, now()).is_ok());
    }
}
