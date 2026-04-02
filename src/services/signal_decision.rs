use std::{collections::HashMap, sync::RwLock};

use tracing::info;

use crate::{
    domain::decision::{DecisionSubmissionResult, SignalDecisionRecord, ValidatedSignalDecision},
    http::middleware::trace_context::RequestContext,
};

#[derive(Default)]
pub struct SignalDecisionService {
    decisions: RwLock<HashMap<String, SignalDecisionRecord>>,
}

impl SignalDecisionService {
    pub fn new() -> Self {
        Self {
            decisions: RwLock::new(HashMap::new()),
        }
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

        self.decisions
            .write()
            .expect("signal decision store write lock should succeed")
            .insert(
                decision.signal_id.clone(),
                SignalDecisionRecord::from_decision(decision),
            );

        DecisionSubmissionResult::from_decision(decision)
    }

    pub fn get_by_signal_id(&self, signal_id: &str) -> Option<SignalDecisionRecord> {
        self.decisions
            .read()
            .expect("signal decision store read lock should succeed")
            .get(signal_id)
            .cloned()
    }
}
