use tracing_subscriber::{fmt, EnvFilter};

use crate::{config::AppConfig, errors::AppError};

pub fn init_tracing(config: &AppConfig) -> Result<(), AppError> {
    let filter = EnvFilter::try_new(config.log_level.clone())
        .map_err(|error| AppError::configuration(format!("invalid LOG_LEVEL: {error}")))?;

    fmt()
        .with_env_filter(filter)
        .json()
        .with_current_span(true)
        .try_init()
        .map_err(|error| {
            AppError::configuration(format!("failed to initialize tracing: {error}"))
        })?;

    Ok(())
}
