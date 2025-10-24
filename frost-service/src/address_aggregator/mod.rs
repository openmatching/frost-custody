pub mod chain_derivation;
pub mod dkg_orchestrator;
pub mod multi_chain_api;

use anyhow::Result;
use poem::{listener::TcpListener, Route, Server};
use poem_openapi::OpenApiService;
use std::sync::Arc;
use tokio::signal;

pub async fn run(
    server_config: crate::config::ServerConfig,
    aggregator_config: crate::config::AggregatorConfig,
    network_config: Option<crate::config::NetworkConfig>,
) -> Result<()> {
    tracing::info!("Signer nodes: {:?}", aggregator_config.signer_nodes);
    tracing::info!(
        "Threshold: {}-of-{}",
        aggregator_config.threshold,
        aggregator_config.signer_nodes.len()
    );

    if let Some(ref net) = network_config {
        tracing::info!("Network: {}", net.network_type);
    }

    // Create multi-chain aggregator API
    let api = multi_chain_api::MultiChainAggregatorApi {
        config: Arc::new(aggregator_config),
        network: Arc::new(network_config),
    };

    // Create API service
    let api_service = OpenApiService::new(api, "FROST Multi-Chain Aggregator", "2.0");

    let ui = api_service.scalar();
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
    .run_with_graceful_shutdown(app, shutdown_signal(), None)
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}
