use std::{collections::HashMap, fs, path::Path};

use crate::errors::AppError;
use jsonschema::JSONSchema;
use serde_json::Value;

pub mod validation;

const REQUIRED_SCHEMAS: [(&str, &str); 2] = [
    ("signal", "signal.json"),
    ("confidence_gate", "confidenceGate.json"),
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
