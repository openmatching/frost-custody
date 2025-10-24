use anyhow::Result;
use std::env;

mod address_aggregator;
mod config;
mod curves;
mod node;
mod signing_aggregator;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load configuration
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
    tracing::info!("Loading configuration from: {}", config_path);

    let config = config::ConfigFile::load(&config_path)?;
    config.validate()?;

    // Dispatch based on role
    match config.server.role.as_str() {
        "node" => {
            tracing::info!("Starting FROST Service in NODE mode");

            let node_config = config.node.expect("Node config validated");
            node::run(config.server, node_config).await
        }
        "address" => {
            tracing::info!("Starting ADDRESS AGGREGATOR mode (DKG orchestration)");

            if let Some(ref net) = config.network {
                tracing::info!("Network type: {}", net.network_type);
            }

            let agg_config = config.aggregator.expect("Aggregator config validated");
            address_aggregator::run(config.server, agg_config, config.network).await
        }
        "signer" => {
            tracing::info!("Starting SIGNING AGGREGATOR mode (FROST signing orchestration)");

            let agg_config = config.aggregator.expect("Aggregator config validated");
            signing_aggregator::run(config.server, agg_config).await
        }
        role => {
            anyhow::bail!("Unknown role: {}", role)
        }
    }
}
