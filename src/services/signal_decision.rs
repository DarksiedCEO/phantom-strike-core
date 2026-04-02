use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
};

use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::{
    domain::decision::{DecisionSubmissionResult, SignalDecisionRecord, ValidatedSignalDecision},
    errors::AppError,
    http::middleware::trace_context::RequestContext,
};

#[derive(Default, Deserialize, Serialize)]
struct SignalDecisionStoreSnapshot {
    decisions: HashMap<String, SignalDecisionRecord>,
}

pub struct SignalDecisionService {
    decisions: RwLock<HashMap<String, SignalDecisionRecord>>,
    store_path: PathBuf,
}

impl SignalDecisionService {
    pub fn new(store_path: PathBuf) -> Result<Self, AppError> {
        let decisions = load_snapshot(&store_path)?.decisions;

        Ok(Self {
            decisions: RwLock::new(decisions),
            store_path,
        })
    }

    pub fn submit(
        &self,
        decision: &ValidatedSignalDecision,
        context: &RequestContext,
    ) -> Result<DecisionSubmissionResult, AppError> {
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

        let mut decisions = self
            .decisions
            .write()
            .expect("signal decision store write lock should succeed");

        decisions.insert(
            decision.signal_id.clone(),
            SignalDecisionRecord::from_decision(decision),
        );
        persist_snapshot(&self.store_path, &decisions)?;

        Ok(DecisionSubmissionResult::from_decision(decision))
    }

    pub fn get_by_signal_id(&self, signal_id: &str) -> Option<SignalDecisionRecord> {
        self.decisions
            .read()
            .expect("signal decision store read lock should succeed")
            .get(signal_id)
            .cloned()
    }
}

fn load_snapshot(store_path: &Path) -> Result<SignalDecisionStoreSnapshot, AppError> {
    if !store_path.exists() {
        return Ok(SignalDecisionStoreSnapshot::default());
    }

    let raw = fs::read_to_string(store_path).map_err(|error| {
        AppError::persistence(format!(
            "failed to read signal decision store at {}: {error}",
            store_path.display()
        ))
    })?;

    serde_json::from_str(&raw).map_err(|error| {
        AppError::persistence(format!(
            "failed to deserialize signal decision store at {}: {error}",
            store_path.display()
        ))
    })
}

fn persist_snapshot(
    store_path: &Path,
    decisions: &HashMap<String, SignalDecisionRecord>,
) -> Result<(), AppError> {
    if let Some(parent) = store_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::persistence(format!(
                "failed to create decision store directory {}: {error}",
                parent.display()
            ))
        })?;
    }

    let temp_path = store_path.with_extension(format!("tmp-{}", Uuid::new_v4()));
    let snapshot = SignalDecisionStoreSnapshot {
        decisions: decisions.clone(),
    };
    let payload = serde_json::to_vec_pretty(&snapshot).map_err(|error| {
        AppError::persistence(format!(
            "failed to serialize signal decision store {}: {error}",
            store_path.display()
        ))
    })?;

    fs::write(&temp_path, payload).map_err(|error| {
        AppError::persistence(format!(
            "failed to write signal decision temp store {}: {error}",
            temp_path.display()
        ))
    })?;

    fs::rename(&temp_path, store_path).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        AppError::persistence(format!(
            "failed to promote signal decision temp store {} -> {}: {error}",
            temp_path.display(),
            store_path.display()
        ))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use uuid::Uuid;

    use crate::{
        domain::decision::ValidatedSignalDecision, http::middleware::trace_context::RequestContext,
    };

    use super::SignalDecisionService;

    fn temp_store_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "phantom-strike-core-{name}-{}.json",
            Uuid::new_v4()
        ))
    }

    fn test_context() -> RequestContext {
        RequestContext {
            request_id: "req-test".to_string(),
            trace_id: "trace-test".to_string(),
            correlation_id: "corr-test".to_string(),
        }
    }

    fn decision(
        signal_id: &str,
        updated_confidence: f64,
        confidence_band: &str,
    ) -> ValidatedSignalDecision {
        ValidatedSignalDecision {
            signal_id: signal_id.to_string(),
            baseline_confidence: 0.62,
            confidence_delta: updated_confidence - 0.62,
            updated_confidence,
            confidence_band: confidence_band.to_string(),
            disposition: "escalate".to_string(),
            reasoning: format!("decision reasoning for {signal_id}"),
            trace_id: "trace-test".to_string(),
            correlation_id: "corr-test".to_string(),
        }
    }

    #[test]
    fn persists_submitted_decision_across_service_restart() {
        let store_path = temp_store_path("persistence");
        let service = SignalDecisionService::new(store_path.clone()).expect("service should load");

        service
            .submit(
                &decision("11111111-1111-4111-8111-111111111111", 0.748, "elevated"),
                &test_context(),
            )
            .expect("submit should persist");

        let restarted =
            SignalDecisionService::new(store_path.clone()).expect("service should reload");
        let record = restarted
            .get_by_signal_id("11111111-1111-4111-8111-111111111111")
            .expect("record should survive restart");

        assert_eq!(record.signal_id, "11111111-1111-4111-8111-111111111111");
        assert_eq!(record.updated_confidence, 0.748);
        assert_eq!(record.confidence_band, "elevated");

        let _ = fs::remove_file(store_path);
    }

    #[test]
    fn upserts_current_decision_for_same_signal_id() {
        let store_path = temp_store_path("upsert");
        let service = SignalDecisionService::new(store_path.clone()).expect("service should load");

        service
            .submit(
                &decision("22222222-2222-4222-8222-222222222222", 0.700, "monitor"),
                &test_context(),
            )
            .expect("first submit should persist");
        service
            .submit(
                &decision("22222222-2222-4222-8222-222222222222", 0.920, "confirmed"),
                &test_context(),
            )
            .expect("second submit should overwrite current record");

        let restarted =
            SignalDecisionService::new(store_path.clone()).expect("service should reload");
        let record = restarted
            .get_by_signal_id("22222222-2222-4222-8222-222222222222")
            .expect("record should exist");

        assert_eq!(record.updated_confidence, 0.920);
        assert_eq!(record.confidence_band, "confirmed");

        let _ = fs::remove_file(store_path);
    }
}
