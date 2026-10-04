//! Page 1 ("STATISTIC OF THE GAME"): real text, so it is the exact source for the game's date
//! and every player's summary row.

use std::{collections::HashSet, sync::LazyLock};

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;

use super::ImportError;

#[derive(Debug, Clone, PartialEq)]
pub struct GameSummary {
    /// Local (venue) time.
    pub started_at: NaiveDateTime,
    pub ended_at: NaiveDateTime,
    pub rows: Vec<SummaryRow>,
}

/// One line of the page-1 table: `Name ID Points Efficiency Frags Deactivated Shots Hits Accuracy`.
#[derive(Debug, Clone, PartialEq)]
pub struct SummaryRow {
    pub name: String,
    /// The player's number in this game (the "ID" column).
    pub number: i32,
    pub points: i32,
    pub efficiency: f64,
    pub frags: i32,
    pub deactivated: i32,
    pub shots: i32,
    pub hits: i32,
    pub accuracy: i32,
}

/// "28 07 2026, 17:42-17:43"
static DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d{1,2})\s+(\d{1,2})\s+(\d{4}),?\s*(\d{1,2}):(\d{2})\s*-\s*(\d{1,2}):(\d{2})")
        .expect("valid regex")
});

/// Name (may contain spaces), then exactly eight numeric columns.
static ROW_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s*(\S.*?)\s+(\d+)\s+(-?\d+)\s+(-?\d+(?:[.,]\d+)?)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s*$",
    )
    .expect("valid regex")
});

pub fn parse(text: &str) -> Result<GameSummary, ImportError> {
    let upper = text.to_uppercase();
    if !upper.contains("STATISTIC OF THE GAME") {
        return Err(ImportError::new(
            "Page 1 is not a game statistics summary (\"STATISTIC OF THE GAME\" not found)",
        ));
    }

    let caps = DATE_RE
        .captures(text)
        .ok_or_else(|| ImportError::new("Page 1: game date and time not found"))?;
    let num = |i: usize| caps[i].parse::<u32>().unwrap_or(u32::MAX);
    let date = NaiveDate::from_ymd_opt(num(3) as i32, num(2), num(1))
        .ok_or_else(|| ImportError::new("Page 1: invalid game date"))?;
    let start = NaiveTime::from_hms_opt(num(4), num(5), 0)
        .ok_or_else(|| ImportError::new("Page 1: invalid start time"))?;
    let end = NaiveTime::from_hms_opt(num(6), num(7), 0)
        .ok_or_else(|| ImportError::new("Page 1: invalid end time"))?;
    let started_at = date.and_time(start);
    let mut ended_at = date.and_time(end);
    if ended_at < started_at {
        // Game ran past midnight.
        ended_at += Duration::days(1);
    }

    let header = text
        .lines()
        .position(|l| {
            let l = l.to_lowercase();
            l.contains("name") && l.contains("points") && l.contains("accuracy")
        })
        .ok_or_else(|| ImportError::new("Page 1: statistics table header not found"))?;

    let mut rows = Vec::new();
    for line in text.lines().skip(header + 1) {
        if line.trim().is_empty() {
            continue;
        }
        let Some(c) = ROW_RE.captures(line) else {
            return Err(ImportError::new(format!(
                "Page 1: unreadable table row: \"{}\"",
                line.trim()
            )));
        };
        let int = |i: usize| -> Result<i32, ImportError> {
            c[i].parse()
                .map_err(|_| ImportError::new(format!("Page 1: bad number in \"{}\"", line.trim())))
        };
        rows.push(SummaryRow {
            name: c[1].trim().to_string(),
            number: int(2)?,
            points: int(3)?,
            efficiency: c[4].replace(',', ".").parse().map_err(|_| {
                ImportError::new(format!("Page 1: bad efficiency in \"{}\"", line.trim()))
            })?,
            frags: int(5)?,
            deactivated: int(6)?,
            shots: int(7)?,
            hits: int(8)?,
            accuracy: int(9)?,
        });
    }

    if rows.is_empty() {
        return Err(ImportError::new(
            "Page 1: the statistics table has no players",
        ));
    }
    let mut numbers = HashSet::new();
    let mut names = HashSet::new();
    for r in &rows {
        if !numbers.insert(r.number) {
            return Err(ImportError::new(format!(
                "Page 1: player ID {} appears twice",
                r.number
            )));
        }
        if !names.insert(r.name.to_lowercase()) {
            return Err(ImportError::new(format!(
                "Page 1: player \"{}\" appears twice",
                r.name
            )));
        }
        if r.accuracy > 100 || r.hits > r.shots {
            return Err(ImportError::new(format!(
                "Page 1: implausible numbers for \"{}\" (hits {} / shots {}, accuracy {}%)",
                r.name, r.hits, r.shots, r.accuracy
            )));
        }
    }

    Ok(GameSummary {
        started_at,
        ended_at,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Text as extracted from the real Total_statistic_17:43.pdf.
    const PAGE1: &str = "\n\n \n\n \n\nSTATISTIC OF THE GAME\n28 07 2026, 17:42-17:43\n\nName ID Points Efficiency Frags Deactivated Shots Hits Accuracy, %\n\nElios 6 206 2.00 2 1 9      4 44\n\nZef 7 201 0.00 0 1 2      1 50\n\nAaron 1 200 0.00 0 0 0      0 0\n\nBella 2 200 0.00 0 0 0      0 0\n\nWolf 3 8 2.00 2 1 15      6 40\n\nIridi 8 0 0.00 0 1 3      0 0\n\nGray 4 0 0.00 0 0 0      0 0\n";

    #[test]
    fn parses_real_summary() {
        let s = parse(PAGE1).unwrap();
        assert_eq!(s.started_at.to_string(), "2026-07-28 17:42:00");
        assert_eq!(s.ended_at.to_string(), "2026-07-28 17:43:00");
        assert_eq!(s.rows.len(), 7);
        assert_eq!(
            s.rows[0],
            SummaryRow {
                name: "Elios".into(),
                number: 6,
                points: 206,
                efficiency: 2.0,
                frags: 2,
                deactivated: 1,
                shots: 9,
                hits: 4,
                accuracy: 44,
            }
        );
        assert_eq!(s.rows[4].name, "Wolf");
        assert_eq!(s.rows[4].shots, 15);
    }

    #[test]
    fn names_with_spaces_and_midnight_rollover() {
        let text = "STATISTIC OF THE GAME\n31 12 2026, 23:58-00:05\nName ID Points Efficiency Frags Deactivated Shots Hits Accuracy, %\nBig Al 3 -5 1.50 3 2 10 5 50\n";
        let s = parse(text).unwrap();
        assert_eq!(s.rows[0].name, "Big Al");
        assert_eq!(s.rows[0].points, -5);
        assert_eq!(s.ended_at.to_string(), "2027-01-01 00:05:00");
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse("some other pdf").is_err());
        let no_rows = "STATISTIC OF THE GAME\n28 07 2026, 17:42-17:43\nName ID Points Efficiency Frags Deactivated Shots Hits Accuracy, %\n";
        assert!(parse(no_rows).is_err());
        let dup = format!("{PAGE1}Elios 9 1 0.00 0 0 0 0 0\n");
        assert!(parse(&dup).is_err());
        let garbage = format!("{PAGE1}Totals 1 2 3\n");
        assert!(
            parse(&garbage).is_err(),
            "an unreadable row must fail, not be skipped"
        );
        let impossible = "STATISTIC OF THE GAME\n28 07 2026, 17:42-17:43\nName ID Points Efficiency Frags Deactivated Shots Hits Accuracy, %\nX 1 0 0.00 0 0 2 5 250\n";
        assert!(parse(impossible).is_err());
    }
}
