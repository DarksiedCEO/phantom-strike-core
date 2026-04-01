use tracing::info;

use crate::{
    domain::decision::{DecisionSubmissionResult, ValidatedSignalDecision},
    http::middleware::trace_context::RequestContext,
};

#[derive(Default)]
pub struct SignalDecisionService;

impl SignalDecisionService {
    pub fn new() -> Self {
        Self
    }

    pub fn submit(
        &self,
        decision: &ValidatedSignalDecision,
        context: &RequestContext,
    ) -> DecisionSubmissionResult {
        info!(
            request_id = context.request_id,
            trace_id = context.trace_id,
            correlation_id = context.correlation_id,
            signal_id = decision.signal_id,
            disposition = decision.disposition,
            confidence_band = decision.confidence_band,
            baseline_confidence = decision.baseline_confidence,
            confidence_delta = decision.confidence_delta,
            updated_confidence = decision.updated_confidence,
            reasoning = decision.reasoning,
            "signal decision accepted after contract validation"
        );

        DecisionSubmissionResult::from_decision(decision)
    }
}
