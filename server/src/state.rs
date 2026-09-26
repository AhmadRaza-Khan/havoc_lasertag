use std::sync::Arc;

use sqlx::PgPool;

use crate::{config::Config, integrations::bookeo::BookeoClient};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub bookeo: Arc<BookeoClient>,
}
