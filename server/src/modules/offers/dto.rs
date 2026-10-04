use chrono::NaiveDate;
use serde::Deserialize;

use crate::{
    error::{AppError, AppResult},
    shared::validation::required_text,
};

const MAX_TITLE: usize = 120;
const MAX_DESCRIPTION: usize = 2000;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Matches title or description, case-insensitive.
    pub search: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Body for both create (POST) and update (PUT).
#[derive(Debug, Deserialize)]
pub struct OfferInput {
    pub title: String,
    pub description: String,
    #[serde(default = "default_active")]
    pub is_active: bool,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
}

fn default_active() -> bool {
    true
}

/// Validated, normalized offer fields ready to persist.
#[derive(Debug)]
pub struct ValidOffer {
    pub title: String,
    pub description: String,
    pub is_active: bool,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
}

impl OfferInput {
    pub fn validate(self) -> AppResult<ValidOffer> {
        let title = required_text("Title", &self.title)?;
        if title.chars().count() > MAX_TITLE {
            return Err(AppError::BadRequest(format!(
                "Title must be at most {MAX_TITLE} characters"
            )));
        }

        let description = required_text("Description", &self.description)?;
        if description.chars().count() > MAX_DESCRIPTION {
            return Err(AppError::BadRequest(format!(
                "Description must be at most {MAX_DESCRIPTION} characters"
            )));
        }

        if let (Some(start), Some(end)) = (self.starts_on, self.ends_on)
            && start > end
        {
            return Err(AppError::BadRequest(
                "End date can't be before the start date".into(),
            ));
        }

        Ok(ValidOffer {
            title,
            description,
            is_active: self.is_active,
            starts_on: self.starts_on,
            ends_on: self.ends_on,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(start: Option<&str>, end: Option<&str>) -> OfferInput {
        OfferInput {
            title: "  Weekend deal ".into(),
            description: " 20% off for groups of 6+ ".into(),
            is_active: true,
            starts_on: start.map(|d| d.parse().unwrap()),
            ends_on: end.map(|d| d.parse().unwrap()),
        }
    }

    #[test]
    fn trims_text() {
        let o = input(None, None).validate().unwrap();
        assert_eq!(o.title, "Weekend deal");
        assert_eq!(o.description, "20% off for groups of 6+");
    }

    #[test]
    fn rejects_blank_and_too_long() {
        let mut i = input(None, None);
        i.title = "  ".into();
        assert!(i.validate().is_err());

        let mut i = input(None, None);
        i.description = "x".repeat(MAX_DESCRIPTION + 1);
        assert!(i.validate().is_err());
    }

    #[test]
    fn dates_must_be_ordered() {
        assert!(
            input(Some("2026-10-05"), Some("2026-10-05"))
                .validate()
                .is_ok()
        );
        assert!(
            input(Some("2026-10-06"), Some("2026-10-05"))
                .validate()
                .is_err()
        );
        assert!(input(None, Some("2026-10-05")).validate().is_ok());
    }
}
