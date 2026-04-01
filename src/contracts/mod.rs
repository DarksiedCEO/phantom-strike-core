use std::{collections::HashMap, fs, path::Path};

use crate::errors::AppError;
use jsonschema::JSONSchema;
use serde_json::Value;

pub mod validation;

const REQUIRED_SCHEMAS: [(&str, &str); 3] = [
    ("signal", "signal.json"),
    ("confidence_gate", "confidenceGate.json"),
    ("signalDecision", "signalDecision.json"),
];

pub struct CompiledSchema {
    pub validator: JSONSchema,
}

pub struct SchemaRegistry {
    schemas: HashMap<String, CompiledSchema>,
}

impl SchemaRegistry {
    pub fn load(schema_dir: &Path) -> Result<Self, AppError> {
        let mut schemas = HashMap::new();

        for (schema_name, file_name) in REQUIRED_SCHEMAS {
            let schema_path = schema_dir.join(file_name);
            let raw = fs::read_to_string(&schema_path).map_err(|error| {
                AppError::schema_loading(format!(
                    "failed reading schema {schema_name} from {}: {error}",
                    schema_path.display()
                ))
            })?;

            let document = serde_json::from_str::<Value>(&raw).map_err(|error| {
                AppError::schema_loading(format!(
                    "failed parsing schema {schema_name} from {}: {error}",
                    schema_path.display()
                ))
            })?;

            let validator = JSONSchema::compile(&document).map_err(|error| {
                AppError::schema_loading(format!(
                    "failed compiling schema {schema_name} from {}: {error}",
                    schema_path.display()
                ))
            })?;

            schemas.insert(schema_name.to_string(), CompiledSchema { validator });
        }

        Ok(Self { schemas })
    }

    pub fn get(&self, schema_name: &str) -> Result<&CompiledSchema, AppError> {
        self.schemas.get(schema_name).ok_or_else(|| {
            AppError::schema_loading(format!("schema {schema_name} is not registered"))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::SchemaRegistry;

    #[test]
    fn fails_startup_when_required_schema_is_missing() {
        let temp_dir = std::env::temp_dir().join(format!(
            "phantom-strike-core-missing-schema-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");

        fs::copy(
            PathBuf::from("/Users/andrelove/IdeaProjects/phantom-strike-contracts/schemas/v1")
                .join("signal.json"),
            temp_dir.join("signal.json"),
        )
        .expect("signal schema should copy");

        fs::copy(
            PathBuf::from("/Users/andrelove/IdeaProjects/phantom-strike-contracts/schemas/v1")
                .join("confidenceGate.json"),
            temp_dir.join("confidenceGate.json"),
        )
        .expect("confidence gate schema should copy");

        let error = match SchemaRegistry::load(&temp_dir) {
            Ok(_) => panic!("schema loading should fail"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("failed reading schema signalDecision"),
            "unexpected error: {error}"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }
}
