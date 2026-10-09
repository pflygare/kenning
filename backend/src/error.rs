use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use ts_rs::TS;

pub type AppResult<T> = Result<T, AppError>;

/// Every error an API handler can return. Each maps to a status code and a
/// stable `code` string the frontend can switch on.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("you need to sign in")]
    Unauthorized,
    #[error("you don't have access to this")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// JSON body of every error response.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ErrorDetail {
    /// Machine-readable, such as `not_found`.
    pub code: String,
    /// Human-readable, safe to show to the user.
    pub message: String,
}

impl AppError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound,
            other => Self::Internal(other.into()),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        let message = match &self {
            // Never leak internals; log them instead.
            Self::Internal(err) => {
                tracing::error!(error = ?err, "internal error");
                "something went wrong".to_string()
            }
            other => other.to_string(),
        };
        let body = ErrorBody {
            error: ErrorDetail {
                code: code.to_string(),
                message,
            },
        };
        (status, Json(body)).into_response()
    }
}
