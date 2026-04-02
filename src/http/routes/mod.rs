use axum::{
    routing::{get, post},
    Router,
};

use super::handlers::{health, signals, version};

pub fn router() -> Router<crate::app::AppState> {
    Router::new()
        .route("/health", get(health::health))
        .route("/version", get(version::version))
        .route("/v1/signals", post(signals::create_signal))
        .route(
            "/v1/signals/:signal_id/decision",
            get(signals::get_signal_decision).post(signals::submit_signal_decision),
        )
        .route(
            "/v1/decisions/by-trace/:trace_id",
            get(signals::get_signal_decision_by_trace_id),
        )
        .route(
            "/v1/decisions/by-correlation/:correlation_id",
            get(signals::get_signal_decision_by_correlation_id),
        )
}
