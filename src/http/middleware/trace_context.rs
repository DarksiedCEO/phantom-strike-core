use axum::{
    http::{header::HeaderName, Request},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RequestContext {
    pub request_id: String,
    pub trace_id: String,
    pub correlation_id: String,
}

pub async fn trace_context_layer(mut request: Request<axum::body::Body>, next: Next) -> Response {
    let request_header = HeaderName::from_static("x-request-id");
    let trace_header = HeaderName::from_static("x-trace-id");
    let correlation_header = HeaderName::from_static("x-correlation-id");

    let request_id = request
        .headers()
        .get(&request_header)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let trace_id = request
        .headers()
        .get(&trace_header)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let correlation_id = request
        .headers()
        .get(&correlation_header)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    request.extensions_mut().insert(RequestContext {
        request_id,
        trace_id,
        correlation_id,
    });

    next.run(request).await
}
