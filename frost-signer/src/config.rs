use anyhow::{Context, Result};
use bitcoin::bip32::DerivationPath;
use bitcoin::hashes::{sha256, Hash};
use bitcoin::Network;
use frost_secp256k1 as frost;
use serde::Deserialize;
use std::fs;
use std::str::FromStr;

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
    pub min_signers: u16,
    pub max_signers: u16,
    /// Hex-encoded FROST key package (secret)
    pub key_package_hex: String,
    /// Hex-encoded FROST public key package (shared)
    pub pubkey_package_hex: String,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

pub struct FrostNode {
    pub network: Network,
    pub node_index: u16,
    pub identifier: frost::Identifier,
    pub key_package: frost::keys::KeyPackage,
    pub pubkey_package: frost::keys::PublicKeyPackage,
}

impl FrostNode {
    pub fn load(path: &str) -> Result<Self> {
        let content =
            fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;

        let config: ConfigFile = toml::from_str(&content).context("Failed to parse config file")?;

        // Parse network
        let network = match config.network.network_type.as_str() {
            "bitcoin" => Network::Bitcoin,
            "testnet" => Network::Testnet,
            "signet" => Network::Signet,
            "regtest" => Network::Regtest,
            _ => anyhow::bail!("Invalid network type: {}", config.network.network_type),
        };

        // Decode key package
        let key_package_bytes = hex::decode(&config.frost.key_package_hex)
            .context("Failed to decode key_package_hex")?;
        let key_package: frost::keys::KeyPackage = serde_json::from_slice(&key_package_bytes)
            .context("Failed to deserialize key package")?;

        // Decode public key package
        let pubkey_package_bytes = hex::decode(&config.frost.pubkey_package_hex)
            .context("Failed to decode pubkey_package_hex")?;
        let pubkey_package: frost::keys::PublicKeyPackage =
            serde_json::from_slice(&pubkey_package_bytes)
                .context("Failed to deserialize pubkey package")?;

        let identifier = *key_package.identifier();

        tracing::info!(
            "Loaded FROST signer node {} for network {:?}",
            config.frost.node_index,
            network
        );
        tracing::info!("FROST identifier: {:?}", identifier);
        let group_pubkey_bytes = pubkey_package
            .verifying_key()
            .serialize()
            .map_err(|e| anyhow::anyhow!("Failed to serialize group public key: {:?}", e))?;
        tracing::info!("Group public key: {}", hex::encode(&group_pubkey_bytes));

        Ok(Self {
            network,
            node_index: config.frost.node_index,
            identifier,
            key_package,
            pubkey_package,
        })
    }

    pub fn get_taproot_address(&self, passphrase: &str) -> Result<String> {
        use bitcoin::secp256k1::Secp256k1;

        // Get the FROST group public key (this is our "xpub" equivalent)
        let group_pubkey = self.pubkey_package.verifying_key();
        let group_pubkey_bytes = group_pubkey
            .serialize()
            .map_err(|e| anyhow::anyhow!("Failed to serialize pubkey: {:?}", e))?;

        // Convert to secp256k1 public key
        let secp = Secp256k1::new();
        let mut secp_pubkey = bitcoin::secp256k1::PublicKey::from_slice(&group_pubkey_bytes)
            .context("Failed to parse group pubkey")?;

        // Hash passphrase to derive deterministic tweak
        let passphrase_hash = sha256::Hash::hash(passphrase.as_bytes());

        // Create secret key from passphrase hash (this becomes our tweak)
        let tweak_key = bitcoin::secp256k1::SecretKey::from_slice(passphrase_hash.as_byte_array())
            .context("Failed to create tweak from passphrase")?;

        // Add the tweak to the public key (standard EC point addition)
        // This gives us: derived_pubkey = group_pubkey + passphrase_hash * G
        secp_pubkey = secp_pubkey
            .add_exp_tweak(&secp, &tweak_key.into())
            .context("Failed to tweak public key")?;

        // Extract x-only pubkey for Taproot (drop the y-coordinate)
        let pubkey_bytes_full = secp_pubkey.serialize();
        let x_only = bitcoin::key::XOnlyPublicKey::from_slice(&pubkey_bytes_full[1..33])
            .context("Failed to create x-only pubkey")?;

        // Create P2TR address
        let address = bitcoin::Address::p2tr_tweaked(
            bitcoin::key::TweakedPublicKey::dangerous_assume_tweaked(x_only),
            self.network,
        );

        Ok(address.to_string())
    }
}

pub fn load_server_config(path: &str) -> Result<ServerConfig> {
    let content =
        fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;

    let config: ConfigFile = toml::from_str(&content).context("Failed to parse config file")?;

    Ok(config.server)
}
