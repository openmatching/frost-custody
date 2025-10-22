mod api;
mod config;
mod crypto;
mod signer;

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
                .unwrap_or_else(|_| "frost_signer=info,poem=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config_path =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "frost-config.toml".to_string());

    tracing::info!("Loading FROST configuration from: {}", config_path);

    let frost_config = Arc::new(config::FrostNode::load(&config_path)?);
    let server_config = config::load_server_config(&config_path)?;

    tracing::info!("Starting FROST signer node {}", frost_config.node_index);
    tracing::info!("Network: {:?}", frost_config.network);
    let group_pubkey_bytes = frost_config
        .pubkey_package
        .verifying_key()
        .serialize()
        .map_err(|e| anyhow::anyhow!("Failed to serialize group public key: {:?}", e))?;
    tracing::info!("Group public key: {}", hex::encode(&group_pubkey_bytes));

    // Create API service
    let api = api::Api {
        config: frost_config.clone(),
    };

    let api_service = OpenApiService::new(api, "FROST Signer", "0.1.0").server(format!(
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
    tracing::info!("FROST signer listening on {}", addr);
    tracing::info!("API documentation: http://{}/docs", addr);
    tracing::info!("OpenAPI spec: http://{}/spec", addr);

    Server::new(TcpListener::bind(&addr)).run(app).await?;

    Ok(())
}
