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
}
