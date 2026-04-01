use std::sync::Arc;

use axum::{middleware, Router};

use crate::{
    config::AppConfig,
    contracts::SchemaRegistry,
    http::{middleware::trace_context::trace_context_layer, routes},
    services::signal_ingestion::SignalIngestionService,
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub schemas: Arc<SchemaRegistry>,
    pub signal_ingestion: Arc<SignalIngestionService>,
}

pub fn build_router(
    config: AppConfig,
    schemas: SchemaRegistry,
    signal_ingestion: SignalIngestionService,
) -> Router {
    let state = AppState {
        config,
        schemas: Arc::new(schemas),
        signal_ingestion: Arc::new(signal_ingestion),
    };

    routes::router()
        .with_state(state)
        .layer(middleware::from_fn(trace_context_layer))
}
