use anyhow::{Context, Result};
use frost_secp256k1_tr as frost;

use super::key_provider::MasterKeyProvider;

/// Run DKG Round 1 with deterministic RNG (using MasterKeyProvider)
pub async fn dkg_part1_with_provider(
    key_provider: &dyn MasterKeyProvider,
    passphrase: &str,
    node_index: u16,
    max_signers: u16,
    min_signers: u16,
) -> Result<(
    frost::keys::dkg::round1::SecretPackage,
    frost::keys::dkg::round1::Package,
)> {
    let mut rng = key_provider.derive_rng(passphrase, "").await?;

    let participant_id = frost::Identifier::try_from(node_index + 1)
        .context("Failed to create participant identifier")?;

    let (secret_package, package) =
        frost::keys::dkg::part1(participant_id, max_signers, min_signers, &mut rng)?;

    Ok((secret_package, package))
}

/// Run DKG Round 1 for ECDSA (Ethereum) with deterministic RNG (using MasterKeyProvider)
pub async fn dkg_part1_ecdsa_with_provider(
    key_provider: &dyn MasterKeyProvider,
    passphrase: &str,
    node_index: u16,
    max_signers: u16,
    min_signers: u16,
) -> Result<(
    frost_secp256k1::keys::dkg::round1::SecretPackage,
    frost_secp256k1::keys::dkg::round1::Package,
)> {
    // Use different prefix to get different keys than Taproot
    let mut rng = key_provider.derive_rng(passphrase, "ecdsa").await?;

    let participant_id = frost_secp256k1::Identifier::try_from(node_index + 1)
        .context("Failed to create ECDSA participant identifier")?;

    let (secret_package, package) =
        frost_secp256k1::keys::dkg::part1(participant_id, max_signers, min_signers, &mut rng)?;

    Ok((secret_package, package))
}

/// Run DKG Round 1 for Ed25519 (Solana) with deterministic RNG (using MasterKeyProvider)
pub async fn dkg_part1_ed25519_with_provider(
    key_provider: &dyn MasterKeyProvider,
    passphrase: &str,
    node_index: u16,
    max_signers: u16,
    min_signers: u16,
) -> Result<(
    frost_ed25519::keys::dkg::round1::SecretPackage,
    frost_ed25519::keys::dkg::round1::Package,
)> {
    // Use different prefix to get different keys than other curves
    let mut rng = key_provider.derive_rng(passphrase, "ed25519").await?;

    let participant_id = frost_ed25519::Identifier::try_from(node_index + 1)
        .context("Failed to create Ed25519 participant identifier")?;

    let (secret_package, package) =
        frost_ed25519::keys::dkg::part1(participant_id, max_signers, min_signers, &mut rng)?;

    Ok((secret_package, package))
}
