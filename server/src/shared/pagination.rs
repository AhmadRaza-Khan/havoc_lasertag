use serde::{Deserialize, Serialize};

const DEFAULT_PER_PAGE: i64 = 20;
const MAX_PER_PAGE: i64 = 100;

/// `?page=1&per_page=20` query params. Pages are 1-based.
#[derive(Debug, Deserialize)]
pub struct PageParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl PageParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn per_page(&self) -> i64 {
        self.per_page
            .unwrap_or(DEFAULT_PER_PAGE)
            .clamp(1, MAX_PER_PAGE)
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }
}

#[derive(Debug, Serialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

impl<T> Paginated<T> {
    pub fn new(items: Vec<T>, total: i64, params: &PageParams) -> Self {
        Self {
            items,
            total,
            page: params.page(),
            per_page: params.per_page(),
        }
    }
}

/// Escapes `%`, `_` and `\` so user input is matched literally inside an ILIKE pattern.
pub fn like_pattern(search: &str) -> String {
    let escaped = search
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_params() {
        let p = PageParams {
            page: Some(0),
            per_page: Some(1000),
        };
        assert_eq!((p.page(), p.per_page(), p.offset()), (1, 100, 0));
        let p = PageParams {
            page: Some(3),
            per_page: None,
        };
        assert_eq!(p.offset(), 40);
    }

    #[test]
    fn escapes_like() {
        assert_eq!(like_pattern("a_b%"), "%a\\_b\\%%");
    }
}
