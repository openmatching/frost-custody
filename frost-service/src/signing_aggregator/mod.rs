//! Signing Aggregator - Orchestrates FROST threshold signing
//!
//! This aggregator:
//! - Receives PSBT/message signing requests from clients
//! - Orchestrates FROST protocol across signer nodes
//! - Returns signed PSBTs/messages
//!
//! Separation from address aggregator:
//! - Address aggregator: DKG orchestration (low risk, generates addresses)
//! - Signing aggregator: FROST signing (high risk, signs transactions)

pub mod signing_api;

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

    // Create signing API
    let api = signing_api::SigningAggregatorApi {
        config: Arc::new(aggregator_config),
    };

    let api_service = OpenApiService::new(api, "FROST Signing Aggregator", "2.0").server(format!(
        "http://{}:{}",
        server_config.host, server_config.port
    ));

    let ui = api_service.rapidoc();
    let spec = api_service.spec_endpoint();

    let app = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/spec", spec);

    tracing::info!(
        "🚀 Signing aggregator listening on {}:{}",
        server_config.host,
        server_config.port
    );
    tracing::info!("   ✍️  POST /api/sign/message {{passphrase, message}}");
    tracing::info!("   ✍️  POST /api/sign/psbt {{psbt, passphrases}}");
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
