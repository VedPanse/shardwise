use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

pub(super) enum ApiError {
    JobNotFound,
    StoreUnavailable,
	InvalidGraph(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::JobNotFound => (
                StatusCode::NOT_FOUND,
                "JOB_NOT_FOUND",
                "No job exists with that ID.",
            ),
            Self::StoreUnavailable => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "The job store is unavailable.",
            ),
			Self::InvalidGraph(message) => (
				StatusCode::UNPROCESSABLE_ENTITY,
				"INVALID_GRAPH",
				message.as_str(),
			),
        };

        (
            status,
            Json(json!({ "error": { "code": code, "message": message } })),
        )
            .into_response()
    }
}
