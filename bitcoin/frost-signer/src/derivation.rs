use anyhow::{Context, Result};
use bitcoin::hashes::{sha256, Hash};
use frost_secp256k1_tr as frost;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

/// Derive deterministic RNG for DKG from master seed and passphrase
///
/// Each node has unique master_seed, so each gets unique RNG.
/// Same passphrase on same node → same RNG → recoverable shares!
pub fn derive_dkg_rng(master_seed: &[u8], passphrase: &str) -> ChaCha20Rng {
    let mut seed_material = master_seed.to_vec();
    seed_material.extend_from_slice(passphrase.as_bytes());
    let seed_hash = sha256::Hash::hash(&seed_material);
    let seed: [u8; 32] = *seed_hash.as_byte_array();

    ChaCha20Rng::from_seed(seed)
}

/// Run DKG Round 1 with deterministic RNG
pub fn dkg_part1(
    master_seed: &[u8],
    passphrase: &str,
    node_index: u16,
) -> Result<(
    frost::keys::dkg::round1::SecretPackage,
    frost::keys::dkg::round1::Package,
)> {
    let mut rng = derive_dkg_rng(master_seed, passphrase);

    let participant_id = frost::Identifier::try_from(node_index + 1)
        .context("Failed to create participant identifier")?;

    let (secret_package, package) = frost::keys::dkg::part1(
        participant_id,
        3, // max_signers
        2, // min_signers
        &mut rng,
    )?;

    Ok((secret_package, package))
}

/// Get or derive FROST shares for passphrase
///
/// Tries cache first, falls back to error (DKG must be triggered via aggregator)
pub fn get_or_derive_share(
    passphrase: &str,
    cache: &crate::storage::ShareStorage,
) -> Result<(frost::keys::KeyPackage, frost::keys::PublicKeyPackage)> {
    // Try cache first
    if let Some(key_pkg) = cache.get_key_package(passphrase)? {
        if let Some(pubkey_pkg) = cache.get_pubkey_package(passphrase)? {
            tracing::debug!("Using cached FROST shares for passphrase");
            return Ok((key_pkg, pubkey_pkg));
        }
    }

    // Cache miss - shares don't exist yet
    tracing::warn!(
        "FROST shares not in cache for passphrase. Use aggregator /api/address/generate to trigger DKG."
    );

    anyhow::bail!(
        "Shares not found. Trigger DKG via aggregator: POST /api/address/generate {{\"passphrase\":\"{}\"}}", 
        passphrase
    )
}
