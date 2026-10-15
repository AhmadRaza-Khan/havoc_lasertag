use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderValue, Method, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use tower_http::{cors::CorsLayer, set_header::SetResponseHeaderLayer, trace::TraceLayer};

use crate::{config::Config, error::AppError, modules, state::AppState};

/// Builds the full HTTP application: all module routers under `/api` plus global middleware.
pub fn build(state: AppState) -> Result<Router, Box<dyn std::error::Error>> {
    let cors = cors_layer(&state.config)?;

    Ok(Router::new()
        .nest("/api", modules::router(&state))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            reject_foreign_origins,
        ))
        .layer(cors)
        // API responses carry personal data: never store them in browser/proxy caches.
        // Only handlers that set their own (versioned avatar images) are cacheable.
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(
            // Log the path only: the chat WebSocket may carry its token in the query string.
            TraceLayer::new_for_http().make_span_with(|req: &Request| {
                tracing::debug_span!("request", method = %req.method(), path = %req.uri().path())
            }),
        )
        .with_state(state))
}

/// CSRF protection for cookie-authenticated requests: browsers always send `Origin` on
/// cross-site POST/PUT/PATCH/DELETE, so a write from a page on another site is refused even
/// if the browser would attach the admin cookie. Requests without `Origin` (the mobile app,
/// curl) are unaffected — they authenticate with Bearer tokens, which other sites can't read.
async fn reject_foreign_origins(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let unsafe_method = !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    if unsafe_method && let Some(origin) = req.headers().get(header::ORIGIN) {
        let allowed = origin
            .to_str()
            .is_ok_and(|o| state.config.cors_origins.iter().any(|a| a == o));
        if !allowed {
            return AppError::Forbidden("Cross-origin request blocked".into()).into_response();
        }
    }
    next.run(req).await
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
