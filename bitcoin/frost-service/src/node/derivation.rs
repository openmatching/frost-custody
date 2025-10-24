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
    max_signers: u16,
    min_signers: u16,
) -> Result<(
    frost::keys::dkg::round1::SecretPackage,
    frost::keys::dkg::round1::Package,
)> {
    let mut rng = derive_dkg_rng(master_seed, passphrase);

    let participant_id = frost::Identifier::try_from(node_index + 1)
        .context("Failed to create participant identifier")?;

    let (secret_package, package) =
        frost::keys::dkg::part1(participant_id, max_signers, min_signers, &mut rng)?;

    Ok((secret_package, package))
}

/// Run DKG Round 1 for ECDSA (Ethereum) with deterministic RNG
pub fn dkg_part1_ecdsa(
    master_seed: &[u8],
    passphrase: &str,
    node_index: u16,
    max_signers: u16,
    min_signers: u16,
) -> Result<(
    frost_secp256k1::keys::dkg::round1::SecretPackage,
    frost_secp256k1::keys::dkg::round1::Package,
)> {
    // Use different seed prefix to get different keys than Taproot
    let mut seed_material = master_seed.to_vec();
    seed_material.extend_from_slice(b"ecdsa:"); // Different from Taproot!
    seed_material.extend_from_slice(passphrase.as_bytes());
    let seed_hash = sha256::Hash::hash(&seed_material);
    let seed: [u8; 32] = *seed_hash.as_byte_array();
    let mut rng = ChaCha20Rng::from_seed(seed);

    let participant_id = frost_secp256k1::Identifier::try_from(node_index + 1)
        .context("Failed to create ECDSA participant identifier")?;

    let (secret_package, package) =
        frost_secp256k1::keys::dkg::part1(participant_id, max_signers, min_signers, &mut rng)?;

    Ok((secret_package, package))
}
