use anyhow::Result;
use frost_secp256k1_tr as frost;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::node::config::FrostNode;

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SigningCommitments {
    pub identifier: String,
    pub commitments: String, // Hex-encoded
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SignatureShare {
    pub identifier: String,
    pub share: String, // Hex-encoded
}

/// Round 1: Generate signing nonces and commitments
pub fn generate_commitments(
    config: &FrostNode,
    passphrase: &str,
) -> Result<(
    frost::Identifier,
    frost::round1::SigningNonces,
    frost::round1::SigningCommitments,
)> {
    let mut rng = rand::rngs::OsRng;
    let key_package = config
        .share_storage
        .get_key_package(passphrase)?
        .ok_or_else(|| anyhow::anyhow!("Key package not found for passphrase"))?;

    let identifier = *key_package.identifier();
    let (nonces, commitments) = frost::round1::commit(key_package.signing_share(), &mut rng);

    Ok((identifier, nonces, commitments))
}

/// Round 2: Sign with nonces and other parties' commitments
pub fn sign_with_commitments(
    config: &FrostNode,
    passphrase: &str,
    message: &[u8],
    nonces: &frost::round1::SigningNonces,
    all_commitments: BTreeMap<frost::Identifier, frost::round1::SigningCommitments>,
) -> Result<frost::round2::SignatureShare> {
    // Create signing package
    let signing_package = frost::SigningPackage::new(all_commitments, message);

    // Generate signature share
    let signature_share = frost::round2::sign(
        &signing_package,
        nonces,
        &config
            .share_storage
            .get_key_package(passphrase)?
            .ok_or_else(|| anyhow::anyhow!("Key package not found for passphrase"))?,
    )?;

    Ok(signature_share)
}

/// Aggregate signature shares into final signature
pub fn aggregate_signature(
    config: &FrostNode,
    passphrase: &str,
    message: &[u8],
    commitments: BTreeMap<frost::Identifier, frost::round1::SigningCommitments>,
    signature_shares: BTreeMap<frost::Identifier, frost::round2::SignatureShare>,
) -> Result<frost::Signature> {
    let signing_package = frost::SigningPackage::new(commitments, message);

    let group_signature = frost::aggregate(
        &signing_package,
        &signature_shares,
        &config
            .share_storage
            .get_pubkey_package(passphrase)?
            .ok_or_else(|| anyhow::anyhow!("Public key package not found for passphrase"))?,
    )?;

    Ok(group_signature)
}

/// Verify a signature
pub fn verify_signature(
    config: &FrostNode,
    passphrase: &str,
    message: &[u8],
    signature: &frost::Signature,
) -> Result<()> {
    config
        .share_storage
        .get_pubkey_package(passphrase)?
        .ok_or_else(|| anyhow::anyhow!("Public key package not found for passphrase"))?
        .verifying_key()
        .verify(message, signature)?;

    Ok(())
}
