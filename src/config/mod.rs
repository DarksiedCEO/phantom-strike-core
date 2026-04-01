use std::{env, path::PathBuf};

use crate::errors::AppError;

const DEFAULT_PORT: u16 = 3000;
const DEFAULT_ENVIRONMENT: &str = "local";
const DEFAULT_LOG_LEVEL: &str = "info";
const DEFAULT_CONTRACTS_SCHEMA_DIR: &str =
    "/Users/andrelove/IdeaProjects/phantom-strike-contracts/schemas/v1";

#[derive(Clone)]
pub struct AppConfig {
    pub service_name: &'static str,
    pub contract_actor_service: &'static str,
    pub environment: String,
    pub log_level: String,
    pub port: u16,
    pub contracts_schema_dir: PathBuf,
    pub contracts_commit: &'static str,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let _ = dotenvy::dotenv();

        let port = env::var("PORT")
            .ok()
            .map(|value| {
                value
                    .parse::<u16>()
                    .map_err(|_| AppError::configuration("PORT must be a valid u16"))
            })
            .transpose()?
            .unwrap_or(DEFAULT_PORT);

        let contracts_schema_dir = env::var("CONTRACTS_SCHEMA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_CONTRACTS_SCHEMA_DIR));

        Ok(Self {
            service_name: "phantom-strike-core",
            contract_actor_service: "core",
            environment: env::var("ENVIRONMENT")
                .unwrap_or_else(|_| DEFAULT_ENVIRONMENT.to_string()),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| DEFAULT_LOG_LEVEL.to_string()),
            port,
            contracts_schema_dir,
            contracts_commit: "3110d87",
        })
    }
}
