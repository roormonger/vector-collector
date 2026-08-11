use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("too many requests")]
    TooManyRequests { retry_after_secs: u64 },
    #[error("service unavailable")]
    Unavailable(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message, retry) = match &self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone(), None),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string(), None),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string(), None),
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string(), None),
            AppError::TooManyRequests { retry_after_secs } => (
                StatusCode::TOO_MANY_REQUESTS,
                self.to_string(),
                Some(*retry_after_secs),
            ),
            AppError::Unavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, m.clone(), Some(2)),
            AppError::Internal(err) => {
                tracing::error!(error = %err, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".into(),
                    None,
                )
            }
        };

        let mut res = Json(json!({ "error": message })).into_response();
        *res.status_mut() = status;
        if let Some(secs) = retry {
            if let Ok(val) = secs.to_string().parse() {
                res.headers_mut().insert("retry-after", val);
            }
        }
        res
    }
}

pub type AppResult<T> = Result<T, AppError>;
