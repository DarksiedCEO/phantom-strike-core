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
pub struct HealthData {
    status: &'static str,
    service: &'static str,
}

pub async fn health(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> Json<SuccessEnvelope<HealthData>> {
    Json(SuccessEnvelope::new(
        HealthData {
            status: "ok",
            service: state.config.service_name,
        },
        ResponseMeta::from_context(&state.config, &context, "health-status", None, Vec::new()),
    ))
}
