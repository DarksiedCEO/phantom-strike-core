use axum::{
    extract::{rejection::JsonRejection, Extension, State},
    Json,
};
use serde_json::json;
use serde_json::Value;

use crate::{
    app::AppState,
    contracts::validation::validate_payload,
    domain::signal::{SignalAcceptance, ValidatedSignal},
    errors::AppError,
    http::{
        middleware::trace_context::RequestContext,
        response::{ResponseMeta, SuccessEnvelope},
    },
};

pub async fn create_signal(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Json<SuccessEnvelope<SignalAcceptance>>, axum::response::Response> {
    let Json(payload) = payload.map_err(|rejection| {
        AppError::validation(
            "INVALID_JSON_BODY",
            "request body must be valid JSON",
            Some(json!({
                "violations": [
                    {
                        "field": "body",
                        "issue": "invalid_json",
                        "expected": "valid JSON document",
                        "received": rejection.body_text()
                    }
                ]
            })),
        )
        .into_response_with_context(&state.config, &context, "signal")
    })?;

    validate_payload(&state.schemas, "signal", &payload)
        .map_err(|error| error.into_response_with_context(&state.config, &context, "signal"))?;

    let signal = ValidatedSignal::from_value(payload)
        .map_err(|error| error.into_response_with_context(&state.config, &context, "signal"))?;
    let data = state.signal_ingestion.accept(&signal, &context);

    Ok(Json(SuccessEnvelope::new(
        data,
        ResponseMeta::from_context(&state.config, &context, "signal", None, Vec::new()),
    )))
}
