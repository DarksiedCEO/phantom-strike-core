use axum::{
    extract::{Extension, State},
    Json,
};
use serde::Serialize;

use crate::{
    app::AppState,
    http::{
        middleware::trace_context::RequestContext,
        response::{ResponseMeta, SuccessEnvelope},
    },
};

#[derive(Serialize)]
pub struct VersionData {
    service: &'static str,
    version: &'static str,
    environment: String,
    contracts_commit: &'static str,
}

pub async fn version(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> Json<SuccessEnvelope<VersionData>> {
    Json(SuccessEnvelope::new(
        VersionData {
            service: state.config.service_name,
            version: env!("CARGO_PKG_VERSION"),
            environment: state.config.environment.clone(),
            contracts_commit: state.config.contracts_commit,
        },
        ResponseMeta::from_context(&state.config, &context, "service-version", None, Vec::new()),
    ))
}
