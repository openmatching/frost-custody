pub mod chain_derivation;
pub mod dkg_orchestrator;
pub mod multi_chain_api;

use anyhow::Result;
use poem::{listener::TcpListener, Route, Server};
use poem_openapi::OpenApiService;
use std::sync::Arc;

pub async fn run(
    server_config: crate::config::ServerConfig,
    aggregator_config: crate::config::AggregatorConfig,
) -> Result<()> {
    tracing::info!("Signer nodes: {:?}", aggregator_config.signer_nodes);
    tracing::info!(
        "Threshold: {}-of-{}",
        aggregator_config.threshold,
        aggregator_config.signer_nodes.len()
    );

    // Create multi-chain aggregator API
    let api = multi_chain_api::MultiChainAggregatorApi {
        config: Arc::new(aggregator_config),
    };

    // Create API service
    let api_service = OpenApiService::new(api, "FROST Multi-Chain Aggregator", "2.0").server(
        format!("http://{}:{}", server_config.host, server_config.port),
    );

    let ui = api_service.rapidoc();
    let spec = api_service.spec_endpoint();

    let app = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/spec", spec);

    tracing::info!(
        "🚀 Multi-chain aggregator listening on {}:{}",
        server_config.host,
        server_config.port
    );
    tracing::info!(
        "   🌍 POST /api/address/generate {{\"chain\": \"bitcoin\", \"passphrase\": \"uuid\"}}"
    );
    tracing::info!("   🌍 GET /api/address?chain=ethereum&passphrase=uuid");
    tracing::info!(
        "   📖 Documentation: http://{}:{}/docs",
        server_config.host,
        server_config.port
    );

    Server::new(TcpListener::bind(format!(
        "{}:{}",
        server_config.host, server_config.port
    )))
    .run(app)
    .await?;

    Ok(())
}
