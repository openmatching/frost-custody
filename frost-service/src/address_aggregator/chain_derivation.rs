//! Chain-specific address derivation logic
//!
//! The aggregator fetches raw public keys from signer nodes and applies
//! chain-specific transformations to derive addresses.
//!
//! This keeps signer nodes chain-agnostic - they only know about curves.

use anyhow::{anyhow, Result};
use bitcoin::Network;
use sha2::Digest;
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

    // IMPORTANT: FROST signs with the UNTWEAKED key
    // For FROST compatibility, we use UNTWEAKED Taproot addresses
    // This is valid but non-standard (BIP 341 allows it for testing)
    //
    // Standard Taproot: Q = P + tagged_hash("TapTweak", P) * G
    // FROST Taproot:    Q = P (no tweak, signs with original key)
    //
    // Trade-off: Simpler FROST implementation, but addresses are identifiable as "raw key"

    let tweaked_key = bitcoin::key::TweakedPublicKey::dangerous_assume_tweaked(x_only);
    let address = bitcoin::Address::p2tr_tweaked(tweaked_key, network);

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
}
