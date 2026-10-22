pub mod app;
pub mod config;
pub mod db;
pub mod error;
pub mod integrations;
pub mod modules;
pub mod shared;
pub mod state;

use std::sync::Arc;

use tracing_subscriber::EnvFilter;

use crate::{
    config::Config,
    integrations::{bookeo::BookeoClient, email::Mailer, expo_push::ExpoPush},
    state::AppState,
};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("server=info,tower_http=info")),
        )
        .init();

    let config = Config::from_env()?;
    let db = db::connect(&config.database_url).await?;
    db::migrate(&db).await?;

    let bind_addr = config.bind_addr.clone();
    let state = AppState {
        db,
        bookeo: Arc::new(BookeoClient::new(&config.bookeo)?),
        skipcash_slot: Arc::new(Default::default()),
        mailer: Arc::new(Mailer::new(config.email.as_ref())?),
        push: Arc::new(ExpoPush::new(
            std::env::var("EXPO_PUSH_URL").ok(),
            std::env::var("EXPO_ACCESS_TOKEN").ok(),
        )?),
        chat: Arc::new(Default::default()),
        login_throttle: Arc::new(Default::default()),
        config: Arc::new(config),
    };
    // Imports interrupted by a restart can't resume: mark them failed, drop their temp files.
    modules::stats::recover(&state).await?;
    // SkipCash credentials are saved by admins in Settings.
    modules::payments::load_skipcash(&state).await?;
    if state.skipcash().is_none() {
        tracing::warn!("SkipCash is not set up (admin panel → Settings): online payment is off");
    }
    if state.config.email.is_none() {
        tracing::warn!("SMTP2GO is not configured: emails are logged, not sent");
    }
    // Confirms payments, records them in Bookeo and cancels overdue unpaid bookings.
    tokio::spawn(modules::payments::worker::run(state.clone()));
    // Bookings canceled in Bookeo are canceled here too (when BOOKEO_WEBHOOK_URL is set).
    tokio::spawn(modules::webhooks::register(state.clone()));
    let app = app::build(state)?;

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("listening on http://{bind_addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
