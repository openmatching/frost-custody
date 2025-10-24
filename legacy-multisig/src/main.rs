mod api;
mod config;
mod signer;

use anyhow::Result;
use poem::listener::TcpListener;
use poem::{Route, Server};
use poem_openapi::OpenApiService;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "consensus_ring=info,poem=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());

    tracing::info!("Loading configuration from: {}", config_path);

    let signer_config = Arc::new(config::SignerNode::load(&config_path)?);
    let server_config = config::load_server_config(&config_path)?;

    tracing::info!(
        "Starting consensus-ring signer node {}",
        signer_config.node_index
    );
    tracing::info!("Network: {:?}", signer_config.network);
    tracing::info!("Account xpub: {}", signer_config.account_xpub);

    // Create API service
    let api = api::Api {
        config: signer_config.clone(),
    };

    let api_service = OpenApiService::new(api, "Consensus Ring", "0.1.0");

    let ui = api_service.scalar();
    let spec = api_service.spec_endpoint();

    let app = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/spec", spec);

    let addr = format!("{}:{}", server_config.host, server_config.port);
    tracing::info!("Server listening on {}", addr);
    tracing::info!("API documentation: http://{}/docs", addr);
    tracing::info!("OpenAPI spec: http://{}/spec", addr);

    Server::new(TcpListener::bind(&addr))
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
