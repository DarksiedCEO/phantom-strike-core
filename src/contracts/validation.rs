use serde_json::{json, Value};

use crate::errors::AppError;

use super::SchemaRegistry;

pub fn validate_payload(
    registry: &SchemaRegistry,
    schema_name: &str,
    payload: &Value,
) -> Result<(), AppError> {
    let schema = registry.get(schema_name)?;

    schema.validator.validate(payload).map_err(|errors| {
        let details = errors
            .map(|error| {
                normalize_violation(
                    payload,
                    &error.to_string(),
                    &error.instance_path.to_string(),
                    &error.schema_path.to_string(),
                )
            })
            .collect::<Vec<_>>();

        AppError::validation(
            "CONTRACT_VALIDATION_FAILED",
            format!("payload failed schema validation for {schema_name}"),
            Some(json!({
                "schema": schema_name,
                "violations": details
            })),
        )
    })
}

fn normalize_violation(
    payload: &Value,
    message: &str,
    instance_path: &str,
    schema_path: &str,
) -> Value {
    if let Some(field) = extract_required_field(message) {
        return json!({
            "field": field,
            "issue": "missing",
            "expected": "required",
            "received": Value::Null
        });
    }

    let field = pointer_field_name(instance_path);
    let received = payload
        .pointer(instance_path)
        .cloned()
        .unwrap_or(Value::Null);

    json!({
        "field": field,
        "issue": normalize_issue(schema_path),
        "expected": normalize_expected(schema_path),
        "received": received
    })
}

fn extract_required_field(message: &str) -> Option<String> {
    message
        .split('"')
        .nth(1)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn pointer_field_name(instance_path: &str) -> String {
    instance_path
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "root".to_string())
}

fn normalize_issue(schema_path: &str) -> &'static str {
    if schema_path.ends_with("/format") {
        "format_mismatch"
    } else if schema_path.ends_with("/type") {
        "type_mismatch"
    } else if schema_path.ends_with("/enum") {
        "enum_mismatch"
    } else if schema_path.ends_with("/minimum") || schema_path.ends_with("/maximum") {
        "range_mismatch"
    } else if schema_path.ends_with("/minLength") || schema_path.ends_with("/maxLength") {
        "length_mismatch"
    } else if schema_path.ends_with("/required") {
        "missing"
    } else {
        "invalid"
    }
}

fn normalize_expected(schema_path: &str) -> String {
    schema_path
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or("constraint")
        .to_string()
}
