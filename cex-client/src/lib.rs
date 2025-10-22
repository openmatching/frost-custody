pub mod frost_client;
pub mod psbt;
pub mod signer_client;

use anyhow::{Context, Result};
use bitcoin::bip32::{DerivationPath, Xpub};
use bitcoin::hashes::{sha256, Hash};
use bitcoin::secp256k1::Secp256k1;
use bitcoin::{Address, Network, PublicKey};
use std::str::FromStr;

// Re-export commonly used items

// Traditional multisig
pub use psbt::{
    add_witness_scripts, build_consolidation_psbt, psbt_from_base64, psbt_to_base64, Utxo,
};
pub use signer_client::{sign_with_threshold, SignerClient};

// FROST threshold
pub use frost_client::{
    frost_sign_message, frost_sign_transaction, FrostNodeClient, FrostSignerClient,
};

/// Convert passphrase to 9-level BIP32 derivation path
///
/// Splits SHA-256 hash of passphrase into 9 indices (each 24-28 bits).
/// This gives full 256-bit keyspace using standard BIP32 non-hardened derivation.
///
/// Compatible with consensus-ring signer-node.
pub fn passphrase_to_derivation_path(passphrase: &str) -> DerivationPath {
    // Hash passphrase to get 256 bits
    let hash = sha256::Hash::hash(passphrase.as_bytes());
    let bytes = hash.as_byte_array();

    // Split into 9 chunks for non-hardened derivation path
    // Each chunk is 24 bits (< 2^31, non-hardened)
    let indices = [
        u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]),
        u32::from_be_bytes([0, bytes[3], bytes[4], bytes[5]]),
        u32::from_be_bytes([0, bytes[6], bytes[7], bytes[8]]),
        u32::from_be_bytes([0, bytes[9], bytes[10], bytes[11]]),
        u32::from_be_bytes([0, bytes[12], bytes[13], bytes[14]]),
        u32::from_be_bytes([0, bytes[15], bytes[16], bytes[17]]),
        u32::from_be_bytes([0, bytes[18], bytes[19], bytes[20]]),
        u32::from_be_bytes([0, bytes[21], bytes[22], bytes[23]]),
        u32::from_be_bytes([0, bytes[24], bytes[25], bytes[26]]),
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

/// Derive 2-of-3 multisig address from passphrases and xpubs
///
/// This allows CEX backend to derive addresses locally without calling signer API.
/// Uses standard BIP32 derivation - compatible with any BIP32 library.
///
/// # Arguments
/// * `xpubs` - 3 account xpubs from signer nodes (at m/48'/0'/0'/2')
/// * `passphrase` - Random UUID or hex string (NOT sequential ID!)
/// * `network` - Bitcoin network
///
/// # Example
/// See `examples/derive_address.rs` for complete usage example.
pub fn derive_multisig_address(
    xpubs: &[Xpub],
    passphrase: &str,
    network: Network,
) -> Result<Address> {
    if xpubs.len() != 3 {
        anyhow::bail!("Must provide exactly 3 xpubs");
    }

    let secp = Secp256k1::new();

    // Convert passphrase to 9-level derivation path
    let path = passphrase_to_derivation_path(passphrase);

    // Derive child pubkeys from all 3 xpubs (standard BIP32!)
    let mut pubkeys = Vec::new();
    for xpub in xpubs {
        let child_xpub = xpub
            .derive_pub(&secp, &path)
            .context("Failed to derive child pubkey")?;

        pubkeys.push(PublicKey::new(child_xpub.public_key));
    }

    // Sort pubkeys for sortedmulti
    pubkeys.sort();

    // Create 2-of-3 multisig witness script
    let script = bitcoin::blockdata::script::Builder::new()
        .push_int(2)
        .push_key(&pubkeys[0])
        .push_key(&pubkeys[1])
        .push_key(&pubkeys[2])
        .push_int(3)
        .push_opcode(bitcoin::blockdata::opcodes::all::OP_CHECKMULTISIG)
        .into_script();

    // Create P2WSH address
    let address = Address::p2wsh(&script, network);

    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passphrase_to_path() {
        let passphrase = "550e8400-e29b-41d4-a716-446655440000";
        let path = passphrase_to_derivation_path(passphrase);

        // Path should have 9 levels
        assert_eq!(path.len(), 9);

        // Same passphrase always gives same path
        let path2 = passphrase_to_derivation_path(passphrase);
        assert_eq!(path.to_string(), path2.to_string());
    }

    #[test]
    fn test_different_passphrases_different_paths() {
        let path1 = passphrase_to_derivation_path("uuid1");
        let path2 = passphrase_to_derivation_path("uuid2");

        assert_ne!(path1.to_string(), path2.to_string());
    }
}
