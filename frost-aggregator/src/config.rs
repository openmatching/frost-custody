use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub frost: FrostConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Deserialize)]
pub struct FrostConfig {
    /// URLs of FROST signer nodes
    pub signer_nodes: Vec<String>,
    /// Threshold (number of nodes needed to sign)
    pub threshold: usize,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

pub struct AggregatorConfig {
    pub signer_nodes: Vec<String>,
    pub threshold: usize,
}

impl AggregatorConfig {
    pub fn load(path: &str) -> Result<(Self, ServerConfig)> {
        let content =
            fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;

        let config: ConfigFile = toml::from_str(&content).context("Failed to parse config file")?;

        if config.frost.signer_nodes.len() < config.frost.threshold {
            anyhow::bail!(
                "Must have at least {} signer nodes for threshold {}",
                config.frost.threshold,
                config.frost.threshold
            );
        }

        tracing::info!(
            "Loaded FROST aggregator config: {} nodes, {}-of-{} threshold",
            config.frost.signer_nodes.len(),
            config.frost.threshold,
            config.frost.signer_nodes.len()
        );

        Ok((
            AggregatorConfig {
                signer_nodes: config.frost.signer_nodes,
                threshold: config.frost.threshold,
            },
            config.server,
        ))
    }
}
