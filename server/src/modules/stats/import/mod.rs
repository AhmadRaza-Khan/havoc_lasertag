//! Reads a game statistics PDF ("Total_statistic_HH:MM.pdf") into structured data.
//!
//! - Page 1 is text: the exact source for the game date and the summary table.
//! - Pages 2+ are scans, one per player, read by OCR. Each is linked to its summary row by the
//!   player's in-game ID (with a name check). Fields that appear on both pages are taken from
//!   page 1; any difference is reported as a warning, never silently.
//!
//! Pure parsing: no database access. CPU-heavy, so call it from a blocking thread.

mod pdf;
mod personal;
mod summary;

use sha2::{Digest, Sha256};

pub use personal::PersonalPage;
pub use summary::{GameSummary, SummaryRow};

use crate::shared::ocr::Ocr;

/// A user-facing reason the file can't be imported.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportError(pub String);

impl ImportError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug)]
pub struct ParsedGame {
    pub summary: GameSummary,
    pub players: Vec<ParsedPlayer>,
    pub scenario: Option<String>,
    /// Identifies the game itself (date, time and every summary row), so the same game is
    /// recognised even if it's exported to a byte-different file.
    pub fingerprint: String,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct ParsedPlayer {
    pub row: SummaryRow,
    /// `None` if the player's page was missing or couldn't be matched.
    pub personal: Option<PersonalStats>,
}

/// Personal-page data kept alongside the summary row. Victim/insulter names are mapped to the
/// exact names from page 1 where possible.
#[derive(Debug, Clone, Default)]
pub struct PersonalStats {
    pub page: u32,
    pub team: Option<String>,
    pub team_score: Option<i32>,
    pub efficiency_kdr: Option<f64>,
    pub captures: Option<i32>,
    pub hits_taken: Option<i32>,
    pub max_combo: Option<i32>,
    pub place_in_team: Option<i32>,
    pub place_in_game: Option<i32>,
    pub nominations: Vec<String>,
    pub victims: Vec<NamedCount>,
    pub insulters: Vec<NamedCount>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NamedCount {
    pub name: String,
    pub count: Option<i32>,
}

pub fn parse(bytes: &[u8], ocr: &Ocr) -> Result<ParsedGame, ImportError> {
    let content = pdf::read(bytes)?;
    let summary = summary::parse(&content.first_page_text)?;

    let mut warnings: Vec<String> = content
        .unreadable_pages
        .iter()
        .map(|(n, why)| format!("Page {n} skipped: {why}"))
        .collect();

    let mut pages = Vec::new();
    for (number, img) in &content.page_images {
        let regions = personal::regions(img.width(), img.height());
        let texts = ocr
            .read_regions(img, &regions)
            .map_err(|e| ImportError::new(format!("Page {number}: OCR failed ({e})")))?;
        pages.push((*number, personal::parse(&texts)));
    }

    let mut parsed = link(summary, pages, &mut warnings);
    parsed.warnings = warnings;
    Ok(parsed)
}

/// Links personal pages to summary rows and cross-checks them. Separate from `parse` so it can
/// be tested without PDFs or OCR.
fn link(
    summary: GameSummary,
    pages: Vec<(u32, PersonalPage)>,
    warnings: &mut Vec<String>,
) -> ParsedGame {
    let names: Vec<String> = summary.rows.iter().map(|r| r.name.clone()).collect();
    let mut personal: Vec<Option<PersonalStats>> = vec![None; summary.rows.len()];
    let mut scenario = None;

    for (page_no, page) in pages {
        let Some(index) = match_row(&summary.rows, &page) else {
            warnings.push(format!(
                "Page {page_no}: could not tell which player it belongs to (read name {:?}, ID {:?}); its extra stats were not saved",
                page.name.as_deref().unwrap_or("?"),
                page.number
            ));
            continue;
        };
        let row = &summary.rows[index];
        if personal[index].is_some() {
            warnings.push(format!(
                "Page {page_no}: {} has more than one page; only the first was used",
                row.name
            ));
            continue;
        }

        cross_check(page_no, row, &page, warnings);
        if scenario.is_none() {
            scenario = page.scenario.clone();
        }
        let map_names = |list: &[(String, Option<i32>)], kind: &str, warnings: &mut Vec<String>| {
            list.iter()
                .map(|(read, count)| {
                    let name = closest_name(&names, read).unwrap_or_else(|| {
                        warnings.push(format!(
                            "Page {page_no}: {kind} \"{read}\" doesn't match any player in this game"
                        ));
                        read.clone()
                    });
                    NamedCount { name, count: *count }
                })
                .collect::<Vec<_>>()
        };
        let victims = map_names(&page.victims, "victim", warnings);
        let insulters = map_names(&page.insulters, "insulter", warnings);

        personal[index] = Some(PersonalStats {
            page: page_no,
            team: page.team.clone(),
            team_score: page.team_score,
            efficiency_kdr: page.efficiency_kdr,
            captures: page.captures,
            hits_taken: page.hits_taken,
            max_combo: page.max_combo,
            place_in_team: page.place_in_team,
            place_in_game: page.place_in_game,
            nominations: page.nominations.clone(),
            victims,
            insulters,
        });
    }

    for (row, p) in summary.rows.iter().zip(&personal) {
        if p.is_none() {
            warnings.push(format!(
                "{} has no readable personal page; only the summary stats were saved",
                row.name
            ));
        }
    }

    let fingerprint = fingerprint(&summary);
    let players = summary
        .rows
        .iter()
        .cloned()
        .zip(personal)
        .map(|(row, personal)| ParsedPlayer { row, personal })
        .collect();

    ParsedGame {
        summary,
        players,
        scenario,
        fingerprint,
        warnings: Vec::new(),
    }
}

/// By in-game ID when the name agrees (allowing OCR slips); otherwise by a confident name match.
fn match_row(rows: &[SummaryRow], page: &PersonalPage) -> Option<usize> {
    let name_ok = |row: &SummaryRow| {
        page.name
            .as_deref()
            .is_none_or(|n| edit_distance(&n.to_lowercase(), &row.name.to_lowercase()) <= 2)
    };
    if let Some(i) = page
        .number
        .and_then(|n| rows.iter().position(|r| r.number == n))
        && name_ok(&rows[i])
    {
        return Some(i);
    }
    let read = page.name.as_deref()?;
    let names: Vec<String> = rows.iter().map(|r| r.name.clone()).collect();
    let name = closest_name(&names, read)?;
    rows.iter().position(|r| r.name == name)
}

/// The page-1 name that `read` most likely is: exact (case-insensitive), else the unique
/// closest within a small edit distance (OCR drops/swaps a character or two).
fn closest_name(names: &[String], read: &str) -> Option<String> {
    let read = read.to_lowercase();
    if let Some(n) = names.iter().find(|n| n.to_lowercase() == read) {
        return Some(n.clone());
    }
    let max = if read.chars().count() <= 4 { 1 } else { 2 };
    let mut best: Vec<(&String, usize)> = names
        .iter()
        .map(|n| (n, edit_distance(&n.to_lowercase(), &read)))
        .filter(|(_, d)| *d <= max)
        .collect();
    best.sort_by_key(|(_, d)| *d);
    match best.as_slice() {
        [(n, _)] => Some((*n).clone()),
        [(n, d1), (_, d2), ..] if d1 < d2 => Some((*n).clone()),
        _ => None, // nothing close, or ambiguous
    }
}

fn cross_check(page_no: u32, row: &SummaryRow, page: &PersonalPage, warnings: &mut Vec<String>) {
    let checks = [
        ("score", page.score, row.points),
        ("shots", page.shots, row.shots),
        ("hits", page.hits, row.hits),
        ("frags", page.frags, row.frags),
        ("deactivated", page.deactivated, row.deactivated),
        ("accuracy", page.accuracy, row.accuracy),
    ];
    for (field, personal, summary) in checks {
        match personal {
            Some(v) if v != summary => warnings.push(format!(
                "{}: {field} on page {page_no} is {v} but the summary says {summary} (summary value used)",
                row.name
            )),
            None => warnings.push(format!(
                "{}: {field} on page {page_no} could not be read (summary value used)",
                row.name
            )),
            _ => {}
        }
    }
}

fn fingerprint(s: &GameSummary) -> String {
    let mut rows: Vec<&SummaryRow> = s.rows.iter().collect();
    rows.sort_by_key(|r| r.number);
    let mut text = format!("{}|{}", s.started_at, s.ended_at);
    for r in rows {
        text.push_str(&format!(
            "|{}:{}:{}:{:.2}:{}:{}:{}:{}:{}",
            r.number,
            r.name.to_lowercase(),
            r.points,
            r.efficiency,
            r.frags,
            r.deactivated,
            r.shots,
            r.hits,
            r.accuracy
        ));
    }
    hex::encode(Sha256::digest(text.as_bytes()))
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, number: i32, points: i32) -> SummaryRow {
        SummaryRow {
            name: name.into(),
            number,
            points,
            efficiency: 0.0,
            frags: 0,
            deactivated: 0,
            shots: 0,
            hits: 0,
            accuracy: 0,
        }
    }

    fn summary() -> GameSummary {
        GameSummary {
            started_at: "2026-07-28T17:42:00".parse().unwrap(),
            ended_at: "2026-07-28T17:43:00".parse().unwrap(),
            rows: vec![row("Elios", 6, 206), row("Wolf", 3, 8), row("Zef", 7, 201)],
        }
    }

    fn page(name: &str, number: Option<i32>, points: i32) -> PersonalPage {
        PersonalPage {
            name: Some(name.into()),
            number,
            score: Some(points),
            shots: Some(0),
            hits: Some(0),
            frags: Some(0),
            deactivated: Some(0),
            accuracy: Some(0),
            ..Default::default()
        }
    }

    #[test]
    fn links_pages_by_id_and_maps_victim_names() {
        let mut wolf = page("Wolf", Some(3), 8);
        wolf.victims = vec![("lios".into(), Some(3)), ("Zef".into(), Some(3))];
        wolf.efficiency_kdr = Some(1.0);
        let mut w = Vec::new();
        let g = link(
            summary(),
            vec![
                (2, page("Elios", Some(6), 206)),
                (3, wolf),
                (4, page("Zef", Some(7), 201)),
            ],
            &mut w,
        );
        assert!(w.is_empty(), "{w:?}");
        let wolf = g.players.iter().find(|p| p.row.name == "Wolf").unwrap();
        let personal = wolf.personal.as_ref().unwrap();
        assert_eq!(personal.page, 3);
        assert_eq!(personal.efficiency_kdr, Some(1.0));
        assert_eq!(
            personal.victims,
            [
                NamedCount {
                    name: "Elios".into(),
                    count: Some(3)
                },
                NamedCount {
                    name: "Zef".into(),
                    count: Some(3)
                }
            ]
        );
    }

    #[test]
    fn falls_back_to_name_when_id_unreadable_and_flags_disagreements() {
        let mut w = Vec::new();
        let mut zef = page("Zef", None, 201);
        zef.accuracy = Some(4);
        let g = link(
            summary(),
            vec![(2, page("Elios", None, 999)), (3, zef)],
            &mut w,
        );
        assert!(g.players[0].personal.is_some(), "Elios matched by name");
        assert!(
            w.iter()
                .any(|m| m.contains("Elios: score on page 2 is 999 but the summary says 206"))
        );
        assert!(w.iter().any(|m| m.contains("Zef: accuracy on page 3 is 4")));
        assert!(
            w.iter()
                .any(|m| m.contains("Wolf has no readable personal page"))
        );
    }

    #[test]
    fn refuses_to_guess_when_id_and_name_disagree() {
        let mut w = Vec::new();
        // ID 3 belongs to Wolf, but the page says "Bella": don't attach it to anyone.
        let g = link(summary(), vec![(2, page("Bella", Some(3), 8))], &mut w);
        assert!(g.players.iter().all(|p| p.personal.is_none()));
        assert!(w.iter().any(|m| m.contains("could not tell which player")));
    }

    #[test]
    fn fingerprint_ignores_row_order_but_not_values() {
        let a = summary();
        let mut b = summary();
        b.rows.reverse();
        assert_eq!(fingerprint(&a), fingerprint(&b));
        b.rows[0].points += 1;
        assert_ne!(fingerprint(&a), fingerprint(&b));
    }

    /// End-to-end on a real report (PDF + OCR). Not run by default (needs the file):
    /// `STATS_PDF=/path/Total_statistic_17:43.pdf cargo test --release real_report -- --ignored`
    #[test]
    #[ignore]
    fn real_report() {
        let path = std::env::var("STATS_PDF").expect("set STATS_PDF");
        let bytes = std::fs::read(path).unwrap();
        let t = std::time::Instant::now();
        let g = parse(&bytes, Ocr::global().unwrap()).unwrap();
        eprintln!("parsed in {:?}; warnings: {:#?}", t.elapsed(), g.warnings);

        assert_eq!(g.summary.started_at.to_string(), "2026-07-28 17:42:00");
        assert_eq!(g.scenario.as_deref(), Some("CS: Detonator"));
        assert_eq!(g.players.len(), 7);
        assert!(
            g.players.iter().all(|p| p.personal.is_some()),
            "every page linked"
        );

        let get = |n: &str| g.players.iter().find(|p| p.row.name == n).unwrap();
        let wolf = get("Wolf");
        assert_eq!((wolf.row.points, wolf.row.efficiency), (8, 2.0));
        let w = wolf.personal.as_ref().unwrap();
        assert_eq!(w.team.as_deref(), Some("RED"));
        assert_eq!(w.team_score, Some(8));
        assert_eq!(
            w.efficiency_kdr,
            Some(1.0),
            "KDR kept separately from efficiency"
        );
        assert_eq!(
            (
                w.captures,
                w.hits_taken,
                w.max_combo,
                w.place_in_team,
                w.place_in_game
            ),
            (Some(0), Some(3), Some(2), Some(1), Some(5))
        );
        assert_eq!(w.nominations, ["First strike", "Stormtrooper", "Miner"]);
        let nc = |n: &str, c| NamedCount {
            name: n.into(),
            count: Some(c),
        };
        assert_eq!(w.victims, [nc("Elios", 3), nc("Zef", 3)]);
        assert_eq!(w.insulters, [nc("Elios", 2), nc("Zef", 1)]);

        let elios = get("Elios").personal.as_ref().unwrap();
        assert_eq!(elios.team.as_deref(), Some("BLUE"));
        assert_eq!(elios.team_score, Some(807));
        assert_eq!(elios.nominations.len(), 6);
        assert_eq!(elios.victims, [nc("Iridi", 2), nc("Wolf", 2)]);

        let iridi = get("Iridi").personal.as_ref().unwrap();
        assert!(iridi.nominations.is_empty() && iridi.victims.is_empty());
        assert_eq!(iridi.insulters, [nc("Elios", 2)]);
        assert_eq!(
            get("Gray").personal.as_ref().unwrap().nominations,
            ["Immortal"]
        );

        // The report itself disagrees with page 1 here; that must surface as warnings.
        assert!(g.warnings.iter().any(|w| w.starts_with("Aaron: accuracy")));
        assert!(g.warnings.iter().any(|w| w.starts_with("Bella: accuracy")));
    }

    #[test]
    fn closest_name_is_conservative() {
        let names = vec!["Elios".to_string(), "Iridi".to_string(), "Zef".to_string()];
        assert_eq!(closest_name(&names, "elios").as_deref(), Some("Elios"));
        assert_eq!(closest_name(&names, "-lios").as_deref(), Some("Elios"));
        assert_eq!(closest_name(&names, "rridi").as_deref(), Some("Iridi"));
        assert_eq!(closest_name(&names, "Bob"), None);
        assert_eq!(closest_name(&names, "Ze"), Some("Zef".into()));
    }
}
