mod app;
mod config;
mod contracts;
mod domain;
mod errors;
mod http;
mod observability;
mod services;

use std::net::SocketAddr;

use app::build_router;
use config::AppConfig;
use contracts::SchemaRegistry;
use observability::init_tracing;
use services::signal_decision::SignalDecisionService;
use services::signal_ingestion::SignalIngestionService;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("fatal startup error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::from_env()?;
    init_tracing(&config)?;

    let schema_registry = SchemaRegistry::load(&config.contracts_schema_dir)?;
    let signal_ingestion = SignalIngestionService::new();
    let signal_decision = SignalDecisionService::new(config.decision_store_path.clone())?;

    let router = build_router(
        config.clone(),
        schema_registry,
        signal_ingestion,
        signal_decision,
    );
    let address = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(address).await?;

    info!(
        service = config.service_name,
        environment = config.environment,
        contracts_schema_dir = %config.contracts_schema_dir.display(),
        decision_store_path = %config.decision_store_path.display(),
        "phantom-strike-core started"
    );

    axum::serve(listener, router).await?;

    Ok(())
}
