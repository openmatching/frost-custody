mod api;
mod config;
mod crypto;
mod derivation;
mod dkg_state;
mod signer;
mod storage;

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

    // Create DKG state for in-progress DKG sessions
    let dkg_state = Arc::new(dkg_state::DkgState::new());
    tracing::info!("✅ DKG state initialized");

    tracing::info!("Starting FROST signer node {}", frost_config.node_index);
    tracing::info!("Network: {:?}", frost_config.network);
    // Create API service
    let api = api::Api {
        config: frost_config.clone(),
        storage: frost_config.share_storage.clone(),
        dkg_state,
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
