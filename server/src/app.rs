use axum::{
    Router,
    http::{HeaderValue, Method, header},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{config::Config, modules, state::AppState};

/// Builds the full HTTP application: all module routers under `/api` plus global middleware.
pub fn build(state: AppState) -> Result<Router, Box<dyn std::error::Error>> {
    let cors = cors_layer(&state.config)?;

    Ok(Router::new()
        .nest("/api", modules::router(&state))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}

fn cors_layer(config: &Config) -> Result<CorsLayer, Box<dyn std::error::Error>> {
    let origins = config
        .cors_origins
        .iter()
        .map(|o| o.parse::<HeaderValue>())
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true))
}
