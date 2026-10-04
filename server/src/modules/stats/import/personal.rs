//! Pages 2+ ("PERSONAL STATISTICS"): one scanned page per player, read by OCR at fixed
//! positions of the report template.

use std::sync::LazyLock;

use regex::Regex;

use crate::shared::ocr::Region;

/// Page size the layout below was measured on; other sizes are scaled proportionally.
const REF_W: f32 = 2263.0;
const REF_H: f32 = 2948.0;

/// Main stats, top to bottom, as printed on the page.
pub const STAT_FIELDS: [&str; 12] = [
    "score",
    "shots",
    "hits",
    "frags",
    "accuracy",
    "captures",
    "hits_taken",
    "deactivated",
    "efficiency_kdr",
    "max_combo",
    "place_in_team",
    "place_in_game",
];
const NOMINATION_SLOTS: usize = 7;
const LIST_ROWS: usize = 9;

/// Where each field sits on the page. `regions()` and `parse()` agree on this order.
struct Layout;

impl Layout {
    const NAME: (f32, f32, f32, f32) = (130., 190., 1000., 270.);
    const TEAM_ID: (f32, f32, f32, f32) = (130., 280., 800., 335.);
    const TEAM_SCORE: (f32, f32, f32, f32) = (770., 460., 1490., 590.);
    const SCENARIO: (f32, f32, f32, f32) = (1500., 460., 2250., 590.);
    /// Stat values are right-aligned at x≈1115; starting at 940 keeps grey captions out.
    const STAT_X: (f32, f32) = (940., 1125.);
    const STAT_FIRST_Y: f32 = 1174.;
    const STAT_STEP: f32 = 130.;
    const STAT_HALF_H: f32 = 40.;
    const NOMINATION_Y: (f32, f32) = (935., 992.);
    const NOMINATION_FIRST_X: f32 = 170.;
    const NOMINATION_STEP: f32 = 320.;
    const NOMINATION_HALF_W: f32 = 160.;
    const VICTIMS_FIRST_Y: f32 = 1154.;
    const INSULTERS_FIRST_Y: f32 = 2110.;
    const LIST_STEP: f32 = 86.;
    const LIST_HALF_H: f32 = 38.;
    const LIST_NAME_X: (f32, f32) = (1145., 1900.);
    const LIST_COUNT_X: (f32, f32) = (2080., 2235.);
}

/// The OCR regions for a page image of `width × height`, in the order `parse` expects.
pub fn regions(width: u32, height: u32) -> Vec<Region> {
    let sx = width as f32 / REF_W;
    let sy = height as f32 / REF_H;
    let r = |(x0, y0, x1, y1): (f32, f32, f32, f32)| Region {
        x0: (x0 * sx).max(0.) as u32,
        y0: (y0 * sy).max(0.) as u32,
        x1: (x1 * sx) as u32,
        y1: (y1 * sy) as u32,
    };

    let mut out = vec![
        r(Layout::NAME),
        r(Layout::TEAM_ID),
        r(Layout::TEAM_SCORE),
        r(Layout::SCENARIO),
    ];
    for i in 0..STAT_FIELDS.len() {
        let c = Layout::STAT_FIRST_Y + Layout::STAT_STEP * i as f32;
        out.push(r((
            Layout::STAT_X.0,
            c - Layout::STAT_HALF_H,
            Layout::STAT_X.1,
            c + Layout::STAT_HALF_H,
        )));
    }
    for k in 0..NOMINATION_SLOTS {
        let cx = Layout::NOMINATION_FIRST_X + Layout::NOMINATION_STEP * k as f32;
        out.push(r((
            cx - Layout::NOMINATION_HALF_W,
            Layout::NOMINATION_Y.0,
            cx + Layout::NOMINATION_HALF_W,
            Layout::NOMINATION_Y.1,
        )));
    }
    for first_y in [Layout::VICTIMS_FIRST_Y, Layout::INSULTERS_FIRST_Y] {
        for k in 0..LIST_ROWS {
            let y = first_y + Layout::LIST_STEP * k as f32;
            let (y0, y1) = (y - Layout::LIST_HALF_H, y + Layout::LIST_HALF_H);
            out.push(r((Layout::LIST_NAME_X.0, y0, Layout::LIST_NAME_X.1, y1)));
            out.push(r((Layout::LIST_COUNT_X.0, y0, Layout::LIST_COUNT_X.1, y1)));
        }
    }
    out
}

/// What one personal page says, after cleaning up OCR output. Unreadable fields are `None`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PersonalPage {
    pub name: Option<String>,
    pub team: Option<String>,
    pub number: Option<i32>,
    pub team_score: Option<i32>,
    pub scenario: Option<String>,
    pub score: Option<i32>,
    pub shots: Option<i32>,
    pub hits: Option<i32>,
    pub frags: Option<i32>,
    pub accuracy: Option<i32>,
    pub captures: Option<i32>,
    pub hits_taken: Option<i32>,
    pub deactivated: Option<i32>,
    pub efficiency_kdr: Option<f64>,
    pub max_combo: Option<i32>,
    pub place_in_team: Option<i32>,
    pub place_in_game: Option<i32>,
    pub nominations: Vec<String>,
    /// (name as read, count)
    pub victims: Vec<(String, Option<i32>)>,
    pub insulters: Vec<(String, Option<i32>)>,
}

static ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bI\s*D\s*[:.]?\s*(\d+)").expect("valid regex"));
static TEAM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*([A-Za-z]{3,})").expect("valid regex"));
static NUMBER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-?\d+(?:\.\d+)?").expect("valid regex"));

/// Turns the OCR text of each region (in `regions()` order) into a page.
pub fn parse(texts: &[Option<String>]) -> PersonalPage {
    let fields: Vec<Option<&str>> = texts
        .iter()
        .map(|t| t.as_deref().map(str::trim).filter(|t| !t.is_empty()))
        .collect();
    let mut cursor = 0;
    let mut next = || {
        let v = fields.get(cursor).copied().flatten();
        cursor += 1;
        v
    };

    let name = next().map(clean_name);
    let team_id = next();
    let team_score = next().and_then(parse_int);
    let scenario = next().map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "));

    let stats: Vec<Option<&str>> = STAT_FIELDS.iter().map(|_| next()).collect();
    let stat = |i: usize| stats[i].and_then(parse_int);

    let nominations: Vec<String> = (0..NOMINATION_SLOTS)
        .filter_map(|_| next())
        .map(clean_name)
        .collect();

    let mut list = || {
        let mut rows = Vec::new();
        for _ in 0..LIST_ROWS {
            let (name, count) = (next(), next());
            if let Some(name) = name {
                rows.push((clean_name(name), count.and_then(parse_int)));
            }
        }
        rows
    };
    let victims = list();
    let insulters = list();

    PersonalPage {
        name,
        team: team_id
            .and_then(|t| TEAM_RE.captures(t))
            .map(|c| c[1].to_uppercase()),
        number: team_id
            .and_then(|t| ID_RE.captures(t))
            .and_then(|c| c[1].parse().ok()),
        team_score,
        scenario,
        score: stat(0),
        shots: stat(1),
        hits: stat(2),
        frags: stat(3),
        accuracy: stat(4),
        captures: stat(5),
        hits_taken: stat(6),
        deactivated: stat(7),
        efficiency_kdr: stats[8].and_then(parse_decimal),
        max_combo: stat(9),
        place_in_team: stat(10),
        place_in_game: stat(11),
        nominations,
        victims,
        insulters,
    }
}

/// Digits only; OCR often reads 0 as the letter O.
fn normalize_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'O' | 'o' => '0',
            ',' => '.',
            c => c,
        })
        .collect()
}

/// The first whole number in a field ("3A" → 3, "O" → 0). Decimals are rejected so a
/// misplaced "1.00" can't silently become 1.
fn parse_int(s: &str) -> Option<i32> {
    let normalized = normalize_digits(s);
    let text = NUMBER_RE.find(&normalized)?.as_str();
    if text.contains('.') {
        return None;
    }
    text.parse().ok()
}

fn parse_decimal(s: &str) -> Option<f64> {
    NUMBER_RE
        .find(&normalize_digits(s))
        .and_then(|m| m.as_str().parse().ok())
}

/// Trims OCR edge noise from names and labels ("-lios", "Wolf ").
fn clean_name(s: &str) -> String {
    s.trim_matches(|c: char| !c.is_alphanumeric())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Region texts in `regions()` order, as OCR returned them for Wolf's real page.
    fn wolf_page() -> Vec<Option<String>> {
        let mut t: Vec<Option<&str>> = vec![
            Some("Wolf"),
            Some("RED  ID 3"),
            Some("8"),
            Some("CS: Detonator"),
        ];
        t.extend(
            [
                "8", "15 ", "6", "2", "40", "O", "3", "1", "1.00", "2 ", "1", "5 ",
            ]
            .map(Some),
        );
        t.extend([
            Some("First strike"),
            Some("Stormtrooper"),
            Some("Miner"),
            None,
            None,
            None,
            None,
        ]);
        let mut victims = vec![Some("Elios"), Some("3A"), Some("Zef "), Some("3")];
        victims.extend(std::iter::repeat_n(None, (LIST_ROWS - 2) * 2));
        let mut insulters = vec![Some("-lios"), Some("2"), Some("Zef"), Some("1")];
        insulters.extend(std::iter::repeat_n(None, (LIST_ROWS - 2) * 2));
        t.extend(victims);
        t.extend(insulters);
        t.into_iter().map(|s| s.map(str::to_string)).collect()
    }

    #[test]
    fn region_count_matches_parse() {
        assert_eq!(regions(2263, 2948).len(), wolf_page().len());
    }

    #[test]
    fn parses_ocr_output() {
        let p = parse(&wolf_page());
        assert_eq!(p.name.as_deref(), Some("Wolf"));
        assert_eq!(p.team.as_deref(), Some("RED"));
        assert_eq!(p.number, Some(3));
        assert_eq!(p.team_score, Some(8));
        assert_eq!(p.scenario.as_deref(), Some("CS: Detonator"));
        assert_eq!(
            (p.score, p.shots, p.hits, p.frags, p.accuracy, p.captures),
            (Some(8), Some(15), Some(6), Some(2), Some(40), Some(0))
        );
        assert_eq!(
            (p.hits_taken, p.deactivated, p.efficiency_kdr, p.max_combo),
            (Some(3), Some(1), Some(1.0), Some(2))
        );
        assert_eq!((p.place_in_team, p.place_in_game), (Some(1), Some(5)));
        assert_eq!(p.nominations, ["First strike", "Stormtrooper", "Miner"]);
        assert_eq!(
            p.victims,
            [("Elios".into(), Some(3)), ("Zef".into(), Some(3))]
        );
        assert_eq!(
            p.insulters,
            [("lios".into(), Some(2)), ("Zef".into(), Some(1))]
        );
    }

    #[test]
    fn cleans_numbers() {
        assert_eq!(parse_int("O"), Some(0));
        assert_eq!(parse_int("3A"), Some(3));
        assert_eq!(parse_int("5 oints"), Some(5));
        assert_eq!(parse_int("1.00"), None, "a decimal is not a count");
        assert_eq!(parse_int("abc"), None);
        assert_eq!(parse_decimal("1,50"), Some(1.5));
    }

    #[test]
    fn reads_ids_in_ocr_variants() {
        for (text, id) in [("BLUE O ID 1", 1), ("RED  ID8", 8), ("BLUE • ID 12", 12)] {
            let mut t = wolf_page();
            t[1] = Some(text.into());
            assert_eq!(parse(&t).number, Some(id), "{text}");
        }
    }

    #[test]
    fn scales_layout_to_other_resolutions() {
        let full = regions(2263, 2948);
        let half = regions(1131, 1474);
        for (f, h) in full.iter().zip(&half) {
            for (a, b) in [(f.x0, h.x0), (f.y0, h.y0), (f.x1, h.x1), (f.y1, h.y1)] {
                assert!((a as i64 / 2 - b as i64).abs() <= 1, "{a} vs {b}");
            }
        }
    }
}
