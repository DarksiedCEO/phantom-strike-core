use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::AppError;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatedSignal {
    pub signal_id: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub status: String,
    pub severity: String,
    pub observed_at: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub scores: Vec<SignalScore>,
    #[serde(default)]
    pub related_source_ids: Vec<String>,
    pub audit: SignalAudit,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignalScore {
    pub score: f64,
    pub confidence_band: String,
    pub rationale: String,
    pub generated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignalAudit {
    pub schema: SignalSchemaDescriptor,
    pub trace: SignalTrace,
    pub recorded_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignalSchemaDescriptor {
    #[serde(rename = "contractVersion")]
    pub contract_version: String,
    #[serde(rename = "schemaName")]
    pub schema_name: String,
    #[serde(rename = "schemaRevision")]
    pub schema_revision: u32,
    #[serde(rename = "packageVersion")]
    pub package_version: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignalTrace {
    pub request_id: String,
    pub trace_id: String,
    pub correlation_id: String,
    pub actor_service: String,
    pub environment: String,
}

#[derive(Serialize)]
pub struct SignalAcceptance {
    pub status: &'static str,
    pub schema: &'static str,
    pub signal_id: String,
    pub accepted_at: String,
}

impl ValidatedSignal {
    pub fn from_value(payload: Value) -> Result<Self, AppError> {
        serde_json::from_value(payload).map_err(|error| {
            AppError::validation(
                "CONTRACT_MAPPING_FAILED",
                "validated payload could not be mapped into the core signal model",
                Some(serde_json::json!({
                    "issue": "mapping_failed",
                    "expected": "ValidatedSignal",
                    "received": error.to_string()
                })),
            )
        })
    }
}

impl SignalAcceptance {
    pub fn from_signal(signal: &ValidatedSignal, accepted_at: String) -> Self {
        Self {
            status: "accepted",
            schema: "signal",
            signal_id: signal.signal_id.clone(),
            accepted_at,
        }
    }
}
