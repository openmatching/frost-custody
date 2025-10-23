// FROST signer node - handles DKG and signing rounds

pub mod api;
pub mod config;
pub mod crypto;
pub mod derivation;
pub mod dkg_state;
pub mod signer;
pub mod storage;

use anyhow::Result;
use bitcoin::Network;
use poem::{listener::TcpListener, Route, Server};
use poem_openapi::OpenApiService;
use std::sync::Arc;

pub async fn run(
    server_config: crate::config::ServerConfig,
    node_config: crate::config::NodeConfig,
    network: Network,
) -> Result<()> {
    // Load node configuration
    let frost_config = config::FrostNode::from_node_config(node_config, network)?;

    tracing::info!("✅ DKG state initialized");
    tracing::info!("Starting FROST signer node {}", frost_config.node_index);
    tracing::info!("Network: {}", network);

    // Create API
    let api_service = OpenApiService::new(
        api::Api {
            config: Arc::new(frost_config.clone()),
            storage: frost_config.share_storage.clone(),
            dkg_state: Arc::new(dkg_state::DkgState::new()),
        },
        "FROST Signer Node",
        "1.0",
    )
    .server(format!("http://{}:{}", server_config.host, server_config.port));

    let ui = api_service.rapidoc();
    let spec = api_service.spec_endpoint();

    let app = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/spec", spec);

    tracing::info!("FROST signer listening on {}:{}", server_config.host, server_config.port);
    tracing::info!("API documentation: http://{}:{}/docs", server_config.host, server_config.port);
    tracing::info!("OpenAPI spec: http://{}:{}/spec", server_config.host, server_config.port);

    Server::new(TcpListener::bind(format!(
        "{}:{}",
        server_config.host, server_config.port
    )))
    .run(app)
    .await?;

    Ok(())
}
