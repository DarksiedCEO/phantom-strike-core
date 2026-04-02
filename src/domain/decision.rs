use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::AppError;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatedSignalDecision {
    pub signal_id: String,
    pub baseline_confidence: f64,
    pub confidence_delta: f64,
    pub updated_confidence: f64,
    pub confidence_band: String,
    pub disposition: String,
    pub reasoning: String,
    pub trace_id: String,
    pub correlation_id: String,
}

#[derive(Serialize)]
pub struct DecisionSubmissionResult {
    pub signal_id: String,
    pub submitted: bool,
    pub target_service: &'static str,
    pub target_endpoint: String,
    pub trace_id: String,
    pub correlation_id: String,
}

#[derive(Clone, Serialize)]
pub struct SignalDecisionRecord {
    pub signal_id: String,
    pub baseline_confidence: f64,
    pub confidence_delta: f64,
    pub updated_confidence: f64,
    pub confidence_band: String,
    pub disposition: String,
    pub reasoning: String,
    pub trace_id: String,
    pub correlation_id: String,
}

impl ValidatedSignalDecision {
    pub fn from_value(payload: Value) -> Result<Self, AppError> {
        serde_json::from_value(payload).map_err(|error| {
            AppError::validation(
                "CONTRACT_MAPPING_FAILED",
                "validated payload could not be mapped into the core decision model",
                Some(serde_json::json!({
                    "issue": "mapping_failed",
                    "expected": "ValidatedSignalDecision",
                    "received": error.to_string()
                })),
            )
        })
    }
}

impl DecisionSubmissionResult {
    pub fn from_decision(decision: &ValidatedSignalDecision) -> Self {
        Self {
            signal_id: decision.signal_id.clone(),
            submitted: true,
            target_service: "core",
            target_endpoint: format!("/v1/signals/{}/decision", decision.signal_id),
            trace_id: decision.trace_id.clone(),
            correlation_id: decision.correlation_id.clone(),
        }
    }
}

impl SignalDecisionRecord {
    pub fn from_decision(decision: &ValidatedSignalDecision) -> Self {
        Self {
            signal_id: decision.signal_id.clone(),
            baseline_confidence: decision.baseline_confidence,
            confidence_delta: decision.confidence_delta,
            updated_confidence: decision.updated_confidence,
            confidence_band: decision.confidence_band.clone(),
            disposition: decision.disposition.clone(),
            reasoning: decision.reasoning.clone(),
            trace_id: decision.trace_id.clone(),
            correlation_id: decision.correlation_id.clone(),
        }
    }
}
