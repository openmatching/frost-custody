use anyhow::{Context, Result};
use bitcoin::Network;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub network: NetworkConfig,
    pub server: ServerConfig,
    #[serde(default)]
    pub node: Option<NodeConfig>,
    #[serde(default)]
    pub aggregator: Option<AggregatorConfig>,
}

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    #[serde(rename = "type")]
    pub network_type: String,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub role: String, // "node", "address", or "signer"
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NodeConfig {
    #[serde(rename = "index")]
    pub node_index: u16,
    pub master_seed_hex: String,
    #[serde(default = "default_storage_path")]
    pub storage_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AggregatorConfig {
    pub signer_nodes: Vec<String>,
    pub threshold: usize,
}

fn default_storage_path() -> String {
    "./data/frost-shares".to_string()
}

impl ConfigFile {
    pub fn load(path: &str) -> Result<Self> {
        let content =
            fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;
        toml::from_str(&content).context("Failed to parse config file")
    }

    pub fn network(&self) -> Result<Network> {
        match self.network.network_type.as_str() {
            "bitcoin" => Ok(Network::Bitcoin),
            "testnet" => Ok(Network::Testnet),
            "signet" => Ok(Network::Signet),
            "regtest" => Ok(Network::Regtest),
            _ => anyhow::bail!("Invalid network type: {}", self.network.network_type),
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self.server.role.as_str() {
            "node" => {
                if self.node.is_none() {
                    anyhow::bail!("Role 'node' requires [node] config section");
                }
            }
            "address" | "signer" => {
                if self.aggregator.is_none() {
                    anyhow::bail!(
                        "Role '{}' requires [aggregator] config section",
                        self.server.role
                    );
                }
            }
            _ => anyhow::bail!(
                "Invalid role: {}. Must be 'node', 'address', or 'signer'",
                self.server.role
            ),
        }
        Ok(())
    }
}
