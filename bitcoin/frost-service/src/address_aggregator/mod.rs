pub mod api;
pub mod dkg_orchestrator;

use anyhow::Result;
use poem::{listener::TcpListener, Route, Server};
use poem_openapi::OpenApiService;
use std::sync::Arc;

pub async fn run(
    server_config: crate::config::ServerConfig,
    aggregator_config: crate::config::AggregatorConfig,
) -> Result<()> {
    tracing::info!("Signer nodes: {:?}", aggregator_config.signer_nodes);
    tracing::info!("Threshold: {}-of-{}", aggregator_config.threshold, aggregator_config.signer_nodes.len());

    // Create API
    let api_service = OpenApiService::new(
        api::Api {
            config: Arc::new(aggregator_config),
        },
        "FROST Address Aggregator",
        "1.0",
    )
    .server(format!("http://{}:{}", server_config.host, server_config.port));

    let ui = api_service.rapidoc();
    let spec = api_service.spec_endpoint();

    let app = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/spec", spec);

    tracing::info!("Address aggregator listening on {}:{}", server_config.host, server_config.port);
    tracing::info!("API documentation: http://{}:{}/docs", server_config.host, server_config.port);

    Server::new(TcpListener::bind(format!(
        "{}:{}",
        server_config.host, server_config.port
    )))
    .run(app)
    .await?;

    Ok(())
}
