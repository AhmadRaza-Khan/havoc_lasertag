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

use crate::{config::Config, integrations::bookeo::BookeoClient, state::AppState};

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
        config: Arc::new(config),
    };
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
