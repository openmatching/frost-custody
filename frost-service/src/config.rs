use anyhow::{Context, Result};
use bitcoin::Network;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub network: Option<NetworkConfig>,
    pub server: ServerConfig,
    #[serde(default)]
    pub node: Option<NodeConfig>,
    #[serde(default)]
    pub aggregator: Option<AggregatorConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NetworkConfig {
    #[serde(rename = "type")]
    pub network_type: String, // "mainnet" or "testnet"

    // Chain-specific network names (optional, defaults based on network_type)
    #[serde(default)]
    pub bitcoin_network: Option<String>, // "mainnet", "testnet", "signet", "regtest"

    #[serde(default)]
    #[allow(dead_code)]
    pub ethereum_network: Option<String>, // "mainnet", "sepolia", "goerli", "holesky"

    #[serde(default)]
    #[allow(dead_code)]
    pub solana_network: Option<String>, // "mainnet-beta", "testnet", "devnet"
}

impl NetworkConfig {
    pub fn bitcoin_network(&self) -> Network {
        let network_str = self
            .bitcoin_network
            .as_deref()
            .unwrap_or(&self.network_type);

        match network_str {
            "mainnet" => Network::Bitcoin,
            "testnet" => Network::Testnet,
            "signet" => Network::Signet,
            "regtest" => Network::Regtest,
            _ => Network::Bitcoin, // Default to mainnet
        }
    }

    #[allow(dead_code)]
    pub fn ethereum_chain_id(&self) -> u64 {
        let network_str = self
            .ethereum_network
            .as_deref()
            .unwrap_or(&self.network_type);

        match network_str {
            "mainnet" => 1,
            "sepolia" => 11155111,
            "goerli" => 5,
            "holesky" => 17000,
            "testnet" => 11155111, // Default testnet = Sepolia
            _ => 1,                // Default to mainnet
        }
    }

    #[allow(dead_code)]
    pub fn solana_cluster(&self) -> &str {
        let network_str = self.solana_network.as_deref().unwrap_or(&self.network_type);

        match network_str {
            "mainnet" => "mainnet-beta",
            "testnet" => "testnet",
            "devnet" => "devnet",
            _ => "mainnet-beta", // Default to mainnet
        }
    }
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
    #[serde(default = "default_max_signers")]
    pub max_signers: u16,
    #[serde(default = "default_min_signers")]
    pub min_signers: u16,
}

impl NodeConfig {
    pub fn master_seed(&self) -> Vec<u8> {
        hex::decode(&self.master_seed_hex).expect("Failed to decode master seed")
    }
}

fn default_max_signers() -> u16 {
    3
}

fn default_min_signers() -> u16 {
    2
}

#[derive(Debug, Deserialize, Clone)]
pub struct AggregatorConfig {
    pub signer_nodes: Vec<String>,
    pub threshold: usize,
}

impl AggregatorConfig {
    pub fn signer_urls(&self) -> &[String] {
        &self.signer_nodes
    }
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
