mod api;
mod config;
mod frost_client;

use anyhow::Result;
use poem::listener::TcpListener;
use poem::{Route, Server};
use poem_openapi::OpenApiService;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "frost_aggregator=info,poem=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config_path = std::env::var("CONFIG_PATH")
        .unwrap_or_else(|_| "aggregator-config.toml".to_string());

    tracing::info!("Loading configuration from: {}", config_path);

    let (aggregator_config, server_config) = config::AggregatorConfig::load(&config_path)?;

    tracing::info!("Starting FROST aggregator service");
    tracing::info!("Signer nodes: {:?}", aggregator_config.signer_nodes);
    tracing::info!(
        "Threshold: {}-of-{}",
        aggregator_config.threshold,
        aggregator_config.signer_nodes.len()
    );

    // Create API service
    let api = api::Api {
        config: Arc::new(aggregator_config),
    };

    let api_service = OpenApiService::new(api, "FROST Aggregator", "0.1.0").server(format!(
        "http://{}:{}",
        server_config.host, server_config.port
    ));

    let ui = api_service.rapidoc();
    let spec = api_service.spec_endpoint();

    let app = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/spec", spec);

    let addr = format!("{}:{}", server_config.host, server_config.port);
    tracing::info!("FROST aggregator listening on {}", addr);
    tracing::info!("API documentation: http://{}/docs", addr);

    Server::new(TcpListener::bind(&addr)).run(app).await?;

    Ok(())
}

