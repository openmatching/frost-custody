use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub network: NetworkConfig,
    pub frost: FrostConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    #[serde(rename = "type")]
    pub network_type: String,
}

#[derive(Debug, Deserialize)]
pub struct FrostConfig {
    pub node_index: u16,
    /// Master seed for share derivation (BACKUP THIS! BIP39 mnemonic recommended)
    pub master_seed_hex: String,
    /// Path to RocksDB storage (cache only, recoverable)
    #[serde(default = "default_storage_path")]
    pub storage_path: String,
    #[serde(default = "default_max_signers")]
    pub max_signers: u16,
    #[serde(default = "default_min_signers")]
    pub min_signers: u16,
}

fn default_max_signers() -> u16 {
    3
}

fn default_min_signers() -> u16 {
    2
}

fn default_storage_path() -> String {
    "./data/frost-shares".to_string()
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Clone)]
pub struct FrostNode {
    pub node_index: u16,
    pub storage_path: String,
    pub max_signers: u16,
    pub min_signers: u16,
    /// Master seed for deterministic derivation (BACKUP THIS!)
    pub master_seed: Vec<u8>,
}

impl FrostNode {
    pub fn from_node_config(node_config: crate::config::NodeConfig) -> Result<Self> {
        // Decode master seed
        let master_seed =
            hex::decode(&node_config.master_seed_hex).context("Invalid master_seed_hex")?;

        tracing::info!("✅ Master seed loaded (can recover all shares from this + passphrases)");

        Ok(Self {
            node_index: node_config.node_index,
            storage_path: node_config.storage_path.clone(),
            max_signers: node_config.max_signers,
            min_signers: node_config.min_signers,
            master_seed,
        })
    }

    pub fn load(path: &str) -> Result<Self> {
        let content =
            fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;

        let config: ConfigFile = toml::from_str(&content).context("Failed to parse config file")?;

        // Decode master seed
        let master_seed = hex::decode(&config.frost.master_seed_hex)
            .context("Failed to decode master_seed_hex")?;

        tracing::info!("✅ Master seed loaded (can recover all shares from this + passphrases)");

        Ok(Self {
            node_index: config.frost.node_index,
            storage_path: config.frost.storage_path.clone(),
            max_signers: config.frost.max_signers,
            min_signers: config.frost.min_signers,
            master_seed,
        })
    }
}

pub fn load_server_config(path: &str) -> Result<ServerConfig> {
    let content =
        fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;

    let config: ConfigFile = toml::from_str(&content).context("Failed to parse config file")?;

    Ok(config.server)
}
