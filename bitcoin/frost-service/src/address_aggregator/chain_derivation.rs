//! Chain-specific address derivation logic
//!
//! The aggregator fetches raw public keys from signer nodes and applies
//! chain-specific transformations to derive addresses.
//!
//! This keeps signer nodes chain-agnostic - they only know about curves.

use anyhow::{anyhow, Result};
use bitcoin::Network;
use sha2::{Digest, Sha256};
use sha3::Keccak256;

/// Supported chains
#[derive(Debug, Clone, Copy)]
pub enum Chain {
    Bitcoin,
    Ethereum,
    Solana,
}

impl Chain {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "bitcoin" | "btc" => Ok(Chain::Bitcoin),
            "ethereum" | "eth" => Ok(Chain::Ethereum),
            "solana" | "sol" => Ok(Chain::Solana),
            _ => Err(anyhow!("Unsupported chain: {}", s)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Chain::Bitcoin => "bitcoin",
            Chain::Ethereum => "ethereum",
            Chain::Solana => "solana",
        }
    }
}

/// Derive Bitcoin Taproot (P2TR) address from secp256k1 public key
pub fn derive_bitcoin_address(pubkey_hex: &str, network: Network) -> Result<String> {
    let pubkey_bytes = hex::decode(pubkey_hex)?;

    if pubkey_bytes.len() != 33 {
        return Err(anyhow!(
            "Invalid secp256k1 pubkey length: {}",
            pubkey_bytes.len()
        ));
    }

    let secp_pubkey = bitcoin::secp256k1::PublicKey::from_slice(&pubkey_bytes)
        .map_err(|e| anyhow!("Failed to parse secp256k1 pubkey: {}", e))?;

    // Extract x-coordinate for Taproot
    let pubkey_full = secp_pubkey.serialize();
    let x_only = bitcoin::key::XOnlyPublicKey::from_slice(&pubkey_full[1..33])
        .map_err(|e| anyhow!("Failed to create x-only pubkey: {}", e))?;

    let address = bitcoin::Address::p2tr_tweaked(
        bitcoin::key::TweakedPublicKey::dangerous_assume_tweaked(x_only),
        network,
    );

    Ok(address.to_string())
}

/// Derive Ethereum address from secp256k1 public key
pub fn derive_ethereum_address(pubkey_hex: &str) -> Result<String> {
    let pubkey_bytes = hex::decode(pubkey_hex)?;

    if pubkey_bytes.len() != 33 {
        return Err(anyhow!(
            "Invalid secp256k1 pubkey length: {}",
            pubkey_bytes.len()
        ));
    }

    let secp_pubkey = bitcoin::secp256k1::PublicKey::from_slice(&pubkey_bytes)
        .map_err(|e| anyhow!("Failed to parse secp256k1 pubkey: {}", e))?;

    // Get uncompressed public key (65 bytes: 0x04 + x + y)
    let uncompressed = secp_pubkey.serialize_uncompressed();

    // Keccak256 hash of the 64-byte public key (skip 0x04 prefix)
    let hash = Keccak256::digest(&uncompressed[1..]);

    // Ethereum address is last 20 bytes of hash
    let address = format!("0x{}", hex::encode(&hash[12..]));

    Ok(address)
}

/// Derive Solana address from Ed25519 public key
pub fn derive_solana_address(pubkey_hex: &str) -> Result<String> {
    let pubkey_bytes = hex::decode(pubkey_hex)?;

    if pubkey_bytes.len() != 32 {
        return Err(anyhow!(
            "Invalid Ed25519 pubkey length: {}",
            pubkey_bytes.len()
        ));
    }

    // Solana address is simply the base58-encoded public key
    let address = bs58::encode(&pubkey_bytes).into_string();

    Ok(address)
}

/// Hash message according to chain-specific rules
pub fn hash_message_for_chain(chain: Chain, message: &[u8]) -> Result<Vec<u8>> {
    match chain {
        Chain::Bitcoin => {
            // Bitcoin signed message format
            let mut msg = Vec::new();
            msg.extend_from_slice(b"\x18Bitcoin Signed Message:\n");
            msg.extend_from_slice(&[message.len() as u8]);
            msg.extend_from_slice(message);

            // Double SHA256
            let hash1 = Sha256::digest(&msg);
            let hash2 = Sha256::digest(hash1);
            Ok(hash2.to_vec())
        }
        Chain::Ethereum => {
            // Ethereum personal_sign format
            let mut msg = Vec::new();
            msg.extend_from_slice(b"\x19Ethereum Signed Message:\n");
            msg.extend_from_slice(message.len().to_string().as_bytes());
            msg.extend_from_slice(message);

            // Keccak256
            let hash = Keccak256::digest(&msg);
            Ok(hash.to_vec())
        }
        Chain::Solana => {
            // Solana signs raw messages (no prefix)
            Ok(message.to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitcoin_address_derivation() {
        // Test with a known secp256k1 public key
        let pubkey = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let address = derive_bitcoin_address(pubkey, Network::Bitcoin).unwrap();
        // Should produce a valid bc1p... address
        assert!(address.starts_with("bc1p"));
    }

    #[test]
    fn test_ethereum_address_derivation() {
        // Test with a known secp256k1 public key
        let pubkey = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let address = derive_ethereum_address(pubkey).unwrap();
        // Should produce a valid 0x... address (42 chars: 0x + 40 hex digits)
        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 42);
    }

    #[test]
    fn test_message_hashing() {
        let message = b"Hello, World!";

        // Bitcoin should double SHA256 with prefix
        let btc_hash = hash_message_for_chain(Chain::Bitcoin, message).unwrap();
        assert_eq!(btc_hash.len(), 32);

        // Ethereum should Keccak256 with prefix
        let eth_hash = hash_message_for_chain(Chain::Ethereum, message).unwrap();
        assert_eq!(eth_hash.len(), 32);

        // Solana should return raw message
        let sol_hash = hash_message_for_chain(Chain::Solana, message).unwrap();
        assert_eq!(sol_hash, message);
    }
}
