use std::sync::{Arc, RwLock};

use sqlx::PgPool;

use crate::{
    config::Config,
    integrations::{
        bookeo::BookeoClient, email::Mailer, expo_push::ExpoPush, skipcash::SkipCashClient,
    },
    modules::chats::hub::ChatHub,
    shared::login_throttle::LoginThrottle,
};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub bookeo: Arc<BookeoClient>,
    /// The SkipCash client for the credentials saved in Settings; replaced when admins change
    /// them. Read it with [`AppState::skipcash`].
    pub skipcash_slot: Arc<RwLock<Option<Arc<SkipCashClient>>>>,
    pub mailer: Arc<Mailer>,
    /// Expo push notifications (chat messages to players who aren't connected).
    pub push: Arc<ExpoPush>,
    /// Live chat connections (in-memory, per process).
    pub chat: Arc<ChatHub>,
    /// Failed-login counters (in-memory, per process).
    pub login_throttle: Arc<LoginThrottle>,
}

impl AppState {
    /// `None` while no SkipCash credentials are saved: online payment is off.
    pub fn skipcash(&self) -> Option<Arc<SkipCashClient>> {
        self.skipcash_slot
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_skipcash(&self, client: Option<SkipCashClient>) {
        *self
            .skipcash_slot
            .write()
            .unwrap_or_else(|e| e.into_inner()) = client.map(Arc::new);
    }
}
