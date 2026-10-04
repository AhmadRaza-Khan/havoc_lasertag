use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    config::Config, integrations::bookeo::BookeoClient, modules::chats::hub::ChatHub,
    shared::login_throttle::LoginThrottle,
};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub bookeo: Arc<BookeoClient>,
    /// Live chat connections (in-memory, per process).
    pub chat: Arc<ChatHub>,
    /// Failed-login counters (in-memory, per process).
    pub login_throttle: Arc<LoginThrottle>,
}
