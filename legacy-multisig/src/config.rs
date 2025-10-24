use anyhow::{Context, Result};
use bip39::Mnemonic;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::hashes::{sha256, Hash};
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

    pub fn derive_pubkey(&self, passphrase: &str) -> Result<bitcoin::PublicKey> {
        let secp = Secp256k1::new();

        // Convert passphrase to derivation path (9 levels for full 256-bit space)
        let path = Self::passphrase_to_derivation_path(passphrase);

        // Standard BIP32 derivation (CEX can do this with xpub!)
        let child_xpub = self
            .account_xpub
            .derive_pub(&secp, &path)
            .context("Failed to derive child pubkey")?;

        Ok(bitcoin::PublicKey::new(child_xpub.public_key))
    }

    pub fn derive_privkey(&self, passphrase: &str) -> Result<bitcoin::PrivateKey> {
        let secp = Secp256k1::new();

        // Convert passphrase to derivation path (9 levels for full 256-bit space)
        let path = Self::passphrase_to_derivation_path(passphrase);

        // Standard BIP32 derivation
        let child_xprv = self
            .account_xprv
            .derive_priv(&secp, &path)
            .context("Failed to derive child privkey")?;

        Ok(bitcoin::PrivateKey::new(
            child_xprv.private_key,
            self.network,
        ))
    }

    pub fn passphrase_to_derivation_path(passphrase: &str) -> DerivationPath {
        // Hash passphrase to get 256 bits
        let hash = sha256::Hash::hash(passphrase.as_bytes());
        let bytes = hash.as_byte_array();

        // Split into 9 chunks for non-hardened derivation path
        // Each chunk is ~28-31 bits, all < 2^31 (non-hardened)
        let indices = [
            u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]), // 24 bits
            u32::from_be_bytes([0, bytes[3], bytes[4], bytes[5]]), // 24 bits
            u32::from_be_bytes([0, bytes[6], bytes[7], bytes[8]]), // 24 bits
            u32::from_be_bytes([0, bytes[9], bytes[10], bytes[11]]), // 24 bits
            u32::from_be_bytes([0, bytes[12], bytes[13], bytes[14]]), // 24 bits
            u32::from_be_bytes([0, bytes[15], bytes[16], bytes[17]]), // 24 bits
            u32::from_be_bytes([0, bytes[18], bytes[19], bytes[20]]), // 24 bits
            u32::from_be_bytes([0, bytes[21], bytes[22], bytes[23]]), // 24 bits
            u32::from_be_bytes([0, bytes[24], bytes[25], bytes[26]]), // 24 bits
            u32::from_be_bytes([0, bytes[27], bytes[28], bytes[29]]), // 24 bits (extra for safety)
        ];

        // Build path: m/i0/i1/i2/i3/i4/i5/i6/i7/i8
        let path_str = format!(
            "m/{}/{}/{}/{}/{}/{}/{}/{}/{}",
            indices[0],
            indices[1],
            indices[2],
            indices[3],
            indices[4],
            indices[5],
            indices[6],
            indices[7],
            indices[8]
        );

        DerivationPath::from_str(&path_str).expect("Valid derivation path")
    }
}

pub fn load_server_config(path: &str) -> Result<ServerConfig> {
    let content =
        fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;

    let config: ConfigFile = toml::from_str(&content).context("Failed to parse config file")?;

    Ok(config.server)
}
