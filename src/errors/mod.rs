use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::{
    config::AppConfig,
    http::{middleware::trace_context::RequestContext, response::ResponseMeta},
};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{message}")]
    Configuration { message: String },
    #[error("{message}")]
    SchemaLoading { message: String },
    #[error("{message}")]
    Validation {
        code: &'static str,
        message: String,
        details: Option<Value>,
    },
}

impl AppError {
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    pub fn schema_loading(message: impl Into<String>) -> Self {
        Self::SchemaLoading {
            message: message.into(),
        }
    }

    pub fn validation(
        code: &'static str,
        message: impl Into<String>,
        details: Option<Value>,
    ) -> Self {
        Self::Validation {
            code,
            message: message.into(),
            details,
        }
    }

    pub fn into_response_with_context(
        self,
        config: &AppConfig,
        context: &RequestContext,
        schema_name: &str,
    ) -> Response {
        let (status, code, message, details, retryable) = match self {
            Self::Configuration { message } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIGURATION_ERROR",
                message,
                None,
                false,
            ),
            Self::SchemaLoading { message } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SCHEMA_LOADING_FAILED",
                message,
                None,
                false,
            ),
            Self::Validation {
                code,
                message,
                details,
            } => (StatusCode::BAD_REQUEST, code, message, details, false),
        };

        let envelope = ErrorEnvelope::from_context(
            config,
            context,
            schema_name,
            code,
            message,
            details,
            retryable,
        );
        (status, Json(envelope)).into_response()
    }
}

#[derive(Serialize)]
struct ErrorEnvelope {
    success: bool,
    error: ErrorBody,
    meta: ResponseMeta,
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
    trace_id: String,
    retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

impl ErrorEnvelope {
    fn from_context(
        config: &AppConfig,
        context: &RequestContext,
        schema_name: &str,
        code: impl Into<String>,
        message: impl Into<String>,
        details: Option<Value>,
        retryable: bool,
    ) -> Self {
        Self {
            success: false,
            error: ErrorBody {
                code: code.into(),
                message: message.into(),
                trace_id: context.trace_id.to_string(),
                retryable,
                details,
            },
            meta: ResponseMeta::from_context(config, context, schema_name, None, Vec::new()),
        }
    }
}
