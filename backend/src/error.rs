use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum AppError {
    Database(String),
    Validation(String),
    NotFound(String),
    Conflict(String),
    Parsing(String),
    #[allow(dead_code)]
    Unknown(String),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(msg)
            | AppError::Validation(msg)
            | AppError::NotFound(msg)
            | AppError::Conflict(msg)
            | AppError::Parsing(msg)
            | AppError::Unknown(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AppError {}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Database(_) | AppError::Unknown(_) | AppError::Parsing(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        let message = self.to_string();
        (status, Json(ErrorBody { message })).into_response()
    }
}
