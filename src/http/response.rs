use serde::Serialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{config::AppConfig, http::middleware::trace_context::RequestContext};

#[derive(Serialize)]
pub struct SuccessEnvelope<T>
where
    T: Serialize,
{
    pub success: bool,
    pub data: T,
    pub meta: ResponseMeta,
}

#[derive(Serialize)]
pub struct ResponseMeta {
    pub audit: AuditMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_band: Option<String>,
    pub warnings: Vec<String>,
    pub trace_id: String,
    pub correlation_id: String,
}

#[derive(Serialize)]
pub struct AuditMetadata {
    pub schema: SchemaDescriptor,
    pub trace: TraceMetadata,
    pub recorded_at: String,
    pub tags: Vec<String>,
}

#[derive(Serialize)]
pub struct SchemaDescriptor {
    #[serde(rename = "contractVersion")]
    pub contract_version: &'static str,
    #[serde(rename = "schemaName")]
    pub schema_name: String,
    #[serde(rename = "schemaRevision")]
    pub schema_revision: u32,
    #[serde(rename = "packageVersion")]
    pub package_version: &'static str,
}

#[derive(Serialize)]
pub struct TraceMetadata {
    pub request_id: String,
    pub trace_id: String,
    pub correlation_id: String,
    pub actor_service: &'static str,
    pub environment: String,
}

impl<T> SuccessEnvelope<T>
where
    T: Serialize,
{
    pub fn new(data: T, meta: ResponseMeta) -> Self {
        Self {
            success: true,
            data,
            meta,
        }
    }
}

impl ResponseMeta {
    pub fn from_context(
        config: &AppConfig,
        context: &RequestContext,
        schema_name: &str,
        confidence_band: Option<&str>,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            audit: AuditMetadata {
                schema: SchemaDescriptor {
                    contract_version: "v1",
                    schema_name: schema_name.to_string(),
                    schema_revision: 0,
                    package_version: env!("CARGO_PKG_VERSION"),
                },
                trace: TraceMetadata {
                    request_id: context.request_id.clone(),
                    trace_id: context.trace_id.clone(),
                    correlation_id: context.correlation_id.clone(),
                    actor_service: config.contract_actor_service,
                    environment: config.environment.clone(),
                },
                recorded_at: now_rfc3339(),
                tags: Vec::new(),
            },
            confidence_band: confidence_band.map(ToOwned::to_owned),
            warnings,
            trace_id: context.trace_id.clone(),
            correlation_id: context.correlation_id.clone(),
        }
    }
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
