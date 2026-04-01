use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing::info;

use crate::{
    domain::signal::{SignalAcceptance, ValidatedSignal},
    http::middleware::trace_context::RequestContext,
};

#[derive(Default)]
pub struct SignalIngestionService;

impl SignalIngestionService {
    pub fn new() -> Self {
        Self
    }

    pub fn accept(&self, signal: &ValidatedSignal, context: &RequestContext) -> SignalAcceptance {
        info!(
            request_id = context.request_id,
            trace_id = context.trace_id,
            correlation_id = context.correlation_id,
            signal_id = signal.signal_id,
            signal_category = signal.category,
            signal_status = signal.status,
            "signal accepted after contract validation"
        );

        SignalAcceptance::from_signal(signal, chrono_like_timestamp())
    }
}

fn chrono_like_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
