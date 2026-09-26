use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    /// The request was valid but an upstream system (e.g. Bookeo) rejected it.
    #[error("{0}")]
    Unprocessable(String),
    /// An upstream system failed or is unreachable. The message is logged, not returned.
    #[error("{0}")]
    BadGateway(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("{0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            AppError::Unprocessable(m) => (StatusCode::UNPROCESSABLE_ENTITY, m.clone()),
            AppError::BadGateway(e) => {
                tracing::error!("upstream error: {e}");
                (
                    StatusCode::BAD_GATEWAY,
                    "Booking service is unavailable, please try again".into(),
                )
            }
            AppError::Database(e) => {
                tracing::error!("database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".into(),
                )
            }
            AppError::Internal(e) => {
                tracing::error!("internal error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".into(),
                )
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
