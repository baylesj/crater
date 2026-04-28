//! `AppError` — unified error type that converts to HTTP responses.

use axum::{http::StatusCode, response::IntoResponse, Json};
use crater_core::CoreError;
use sc_client::ScError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("service unavailable: {0}")]
    #[allow(dead_code)]
    ServiceUnavailable(String),

    #[error(transparent)]
    Core(#[from] CoreError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, code) = match &self {
            AppError::NotFound                 => (StatusCode::NOT_FOUND,            "not_found"),
            AppError::BadRequest(_)            => (StatusCode::BAD_REQUEST,          "bad_request"),
            AppError::ServiceUnavailable(_)    => (StatusCode::SERVICE_UNAVAILABLE,  "service_unavailable"),
            AppError::Core(CoreError::NotFound) => (StatusCode::NOT_FOUND,           "not_found"),
            AppError::Core(CoreError::InvalidCron { .. }) => (StatusCode::BAD_REQUEST, "invalid_cron"),
            AppError::Core(CoreError::InvalidRanking(_)) => (StatusCode::BAD_REQUEST, "invalid_ranking"),
            AppError::Core(CoreError::Sc(ScError::AuthExpired)) => (StatusCode::BAD_GATEWAY, "sc_auth_expired"),
            AppError::Core(CoreError::Sc(ScError::RateLimited)) => (StatusCode::TOO_MANY_REQUESTS, "sc_rate_limited"),
            _                                  => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        let body = serde_json::json!({
            "error":   code,
            "message": self.to_string(),
        });

        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, AppError>;
