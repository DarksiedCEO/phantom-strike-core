use axum::{
    extract::{rejection::JsonRejection, Extension, Path, State},
    Json,
};
use serde_json::json;
use serde_json::Value;

use crate::{
    app::AppState,
    contracts::validation::validate_payload,
    domain::{
        decision::{DecisionSubmissionResult, SignalDecisionRecord, ValidatedSignalDecision},
        signal::{SignalAcceptance, ValidatedSignal},
    },
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

pub async fn submit_signal_decision(
    Path(path_signal_id): Path<String>,
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Json<SuccessEnvelope<DecisionSubmissionResult>>, axum::response::Response> {
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
        .into_response_with_context(&state.config, &context, "signalDecision")
    })?;

    validate_payload(&state.schemas, "signalDecision", &payload).map_err(|error| {
        error.into_response_with_context(&state.config, &context, "signalDecision")
    })?;

    let decision = ValidatedSignalDecision::from_value(payload).map_err(|error| {
        error.into_response_with_context(&state.config, &context, "signalDecision")
    })?;

    if decision.signal_id != path_signal_id {
        return Err(AppError::validation(
            "SIGNAL_ID_MISMATCH",
            "path signal_id must match body signal_id",
            Some(json!({
                "violations": [
                    {
                        "field": "signal_id",
                        "issue": "mismatch",
                        "expected": path_signal_id,
                        "received": decision.signal_id
                    }
                ]
            })),
        )
        .into_response_with_context(&state.config, &context, "signalDecision"));
    }

    let data = state.signal_decision.submit(&decision, &context);
    let data = data.map_err(|error| {
        error.into_response_with_context(&state.config, &context, "decisionSubmissionResult")
    })?;

    Ok(Json(SuccessEnvelope::new(
        data,
        ResponseMeta::from_context(
            &state.config,
            &context,
            "decisionSubmissionResult",
            Some(&decision.confidence_band),
            Vec::new(),
        ),
    )))
}

pub async fn get_signal_decision(
    Path(signal_id): Path<String>,
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> Result<Json<SuccessEnvelope<SignalDecisionRecord>>, axum::response::Response> {
    let record = state
        .signal_decision
        .get_by_signal_id(&signal_id)
        .ok_or_else(|| {
            AppError::not_found(
                "SIGNAL_DECISION_NOT_FOUND",
                "no decision record found for the requested signal_id",
                Some(json!({
                    "lookup": {
                        "field": "signal_id",
                        "value": signal_id
                    }
                })),
            )
            .into_response_with_context(
                &state.config,
                &context,
                "signalDecisionRecord",
            )
        })?;
    let confidence_band = record.confidence_band.clone();

    Ok(Json(SuccessEnvelope::new(
        record,
        ResponseMeta::from_context(
            &state.config,
            &context,
            "signalDecisionRecord",
            Some(&confidence_band),
            Vec::new(),
        ),
    )))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use crate::{
        app::build_router,
        config::AppConfig,
        contracts::SchemaRegistry,
        services::{
            signal_decision::SignalDecisionService, signal_ingestion::SignalIngestionService,
        },
    };

    fn test_contracts_schema_dir() -> PathBuf {
        std::env::var("CONTRACTS_SCHEMA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from("/Users/andrelove/IdeaProjects/phantom-strike-contracts/schemas/v1")
            })
    }

    fn test_config() -> AppConfig {
        AppConfig {
            service_name: "phantom-strike-core",
            contract_actor_service: "core",
            environment: "test".to_string(),
            log_level: "debug".to_string(),
            port: 3000,
            contracts_schema_dir: test_contracts_schema_dir(),
            decision_store_path: std::env::temp_dir().join(format!(
                "phantom-strike-core-handler-test-{}.json",
                uuid::Uuid::new_v4()
            )),
            contracts_commit: "528603a",
        }
    }

    fn build_test_app() -> axum::Router {
        let config = test_config();
        let decision_store_path = config.decision_store_path.clone();

        build_router(
            config,
            SchemaRegistry::load(&test_contracts_schema_dir()).expect("schemas should load"),
            SignalIngestionService::new(),
            SignalDecisionService::new(decision_store_path).expect("decision service should load"),
        )
    }

    fn valid_decision_payload(signal_id: &str) -> Value {
        json!({
            "signal_id": signal_id,
            "baseline_confidence": 0.62,
            "confidence_delta": 0.128,
            "updated_confidence": 0.748,
            "confidence_band": "elevated",
            "disposition": "escalate",
            "reasoning": "Supporting evidence outweighed contradiction after adversarial penalty.",
            "trace_id": "trace-001-alpha",
            "correlation_id": "e5b11411-2732-486f-9d0a-f4144ea20395"
        })
    }

    #[tokio::test]
    async fn accepts_valid_signal_decision_payload() {
        let app = build_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/v1/signals/df1eab71-aa5f-4ce2-9915-64ccf314e3b9/decision")
            .header("content-type", "application/json")
            .body(Body::from(
                valid_decision_payload("df1eab71-aa5f-4ce2-9915-64ccf314e3b9").to_string(),
            ))
            .expect("request should build");

        let response = app.oneshot(request).await.expect("response should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json: Value = serde_json::from_slice(&body).expect("body should be json");

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["submitted"], true);
        assert_eq!(json["data"]["target_service"], "core");
        assert_eq!(json["meta"]["confidence_band"], "elevated");
    }

    #[tokio::test]
    async fn rejects_invalid_signal_decision_payload() {
        let app = build_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/v1/signals/df1eab71-aa5f-4ce2-9915-64ccf314e3b9/decision")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "signal_id": "df1eab71-aa5f-4ce2-9915-64ccf314e3b9" }).to_string(),
            ))
            .expect("request should build");

        let response = app.oneshot(request).await.expect("response should succeed");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json: Value = serde_json::from_slice(&body).expect("body should be json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "CONTRACT_VALIDATION_FAILED");
    }

    #[tokio::test]
    async fn rejects_signal_id_mismatch() {
        let app = build_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/v1/signals/11111111-1111-1111-1111-111111111111/decision")
            .header("content-type", "application/json")
            .body(Body::from(
                valid_decision_payload("df1eab71-aa5f-4ce2-9915-64ccf314e3b9").to_string(),
            ))
            .expect("request should build");

        let response = app.oneshot(request).await.expect("response should succeed");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json: Value = serde_json::from_slice(&body).expect("body should be json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "SIGNAL_ID_MISMATCH");
    }

    #[tokio::test]
    async fn retrieves_persisted_signal_decision() {
        let app = build_test_app();

        let submit_request = Request::builder()
            .method("POST")
            .uri("/v1/signals/df1eab71-aa5f-4ce2-9915-64ccf314e3b9/decision")
            .header("content-type", "application/json")
            .body(Body::from(
                valid_decision_payload("df1eab71-aa5f-4ce2-9915-64ccf314e3b9").to_string(),
            ))
            .expect("request should build");

        let submit_response = app
            .clone()
            .oneshot(submit_request)
            .await
            .expect("submit should succeed");
        assert_eq!(submit_response.status(), StatusCode::OK);

        let get_request = Request::builder()
            .method("GET")
            .uri("/v1/signals/df1eab71-aa5f-4ce2-9915-64ccf314e3b9/decision")
            .body(Body::empty())
            .expect("request should build");

        let response = app
            .oneshot(get_request)
            .await
            .expect("response should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json: Value = serde_json::from_slice(&body).expect("body should be json");

        assert_eq!(json["success"], true);
        assert_eq!(
            json["data"]["signal_id"],
            "df1eab71-aa5f-4ce2-9915-64ccf314e3b9"
        );
        assert_eq!(json["data"]["disposition"], "escalate");
        assert_eq!(json["data"]["confidence_band"], "elevated");
        assert_eq!(json["meta"]["confidence_band"], "elevated");
    }

    #[tokio::test]
    async fn returns_not_found_for_missing_signal_decision() {
        let app = build_test_app();

        let request = Request::builder()
            .method("GET")
            .uri("/v1/signals/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa/decision")
            .body(Body::empty())
            .expect("request should build");

        let response = app.oneshot(request).await.expect("response should succeed");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json: Value = serde_json::from_slice(&body).expect("body should be json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "SIGNAL_DECISION_NOT_FOUND");
    }
}
