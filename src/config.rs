use anyhow::{Context, Result};
use bip39::Mnemonic;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Network;
use serde::Deserialize;
use std::fs;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub network: NetworkConfig,
    pub signer: SignerConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    #[serde(rename = "type")]
    pub network_type: String,
}

#[derive(Debug, Deserialize)]
pub struct SignerConfig {
    pub node_index: u8,
    pub mnemonic: String,
    pub xpubs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

pub struct SignerNode {
    pub network: Network,
    pub node_index: u8,
    pub account_xprv: Xpriv,
    pub account_xpub: Xpub,
    pub all_xpubs: Vec<Xpub>,
}

impl SignerNode {
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

        // Validate node_index
        if config.signer.node_index > 2 {
            anyhow::bail!("node_index must be 0, 1, or 2");
        }

        // Parse mnemonic
        let mnemonic = Mnemonic::parse(&config.signer.mnemonic).context("Invalid mnemonic")?;

        // Derive master key
        let seed = mnemonic.to_seed("");
        let secp = Secp256k1::new();
        let master_xprv =
            Xpriv::new_master(network, &seed).context("Failed to derive master key")?;

        // Derive account xprv at m/48'/0'/0'/2'
        let account_path =
            DerivationPath::from_str("m/48'/0'/0'/2'").context("Invalid derivation path")?;
        let account_xprv = master_xprv
            .derive_priv(&secp, &account_path)
            .context("Failed to derive account key")?;

        // Get account xpub
        let account_xpub = Xpub::from_priv(&secp, &account_xprv);

        // Parse all xpubs
        if config.signer.xpubs.len() != 3 {
            anyhow::bail!("Must provide exactly 3 xpubs");
        }

        let all_xpubs: Vec<Xpub> = config
            .signer
            .xpubs
            .iter()
            .map(|s| Xpub::from_str(s).context("Invalid xpub"))
            .collect::<Result<_>>()?;

        tracing::info!(
            "Loaded signer node {} for network {:?}",
            config.signer.node_index,
            network
        );
        tracing::info!("Account xpub: {}", account_xpub);

        Ok(Self {
            network,
            node_index: config.signer.node_index,
            account_xprv,
            account_xpub,
            all_xpubs,
        })
    }

    pub fn derive_pubkey(&self, user_id: u64) -> Result<bitcoin::PublicKey> {
        let secp = Secp256k1::new();

        // Derive child key at non-hardened path
        let path = DerivationPath::from_str(&format!("m/{}", user_id))
            .context("Invalid derivation path")?;

        let child_xpub = self
            .account_xpub
            .derive_pub(&secp, &path)
            .context("Failed to derive child pubkey")?;

        Ok(bitcoin::PublicKey::new(child_xpub.public_key))
    }

    pub fn derive_privkey(&self, user_id: u64) -> Result<bitcoin::PrivateKey> {
        let secp = Secp256k1::new();

        // Derive child key at non-hardened path
        let path = DerivationPath::from_str(&format!("m/{}", user_id))
            .context("Invalid derivation path")?;

        let child_xprv = self
            .account_xprv
            .derive_priv(&secp, &path)
            .context("Failed to derive child privkey")?;

        Ok(bitcoin::PrivateKey::new(
            child_xprv.private_key,
            self.network,
        ))
    }
}

pub fn load_server_config(path: &str) -> Result<ServerConfig> {
    let content =
        fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;

    let config: ConfigFile = toml::from_str(&content).context("Failed to parse config file")?;

    Ok(config.server)
}
