use anyhow::Result;
use frost_secp256k1 as frost;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::config::FrostNode;

#[derive(Debug, Serialize, Deserialize)]
pub struct SigningCommitments {
    pub identifier: String,
    pub commitments: String, // Hex-encoded
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureShare {
    pub identifier: String,
    pub share: String, // Hex-encoded
}

/// Round 1: Generate signing nonces and commitments
pub fn generate_commitments(
    config: &FrostNode,
) -> Result<(
    frost::round1::SigningNonces,
    frost::round1::SigningCommitments,
)> {
    let mut rng = rand::rngs::OsRng;
    let (nonces, commitments) = frost::round1::commit(config.key_package.signing_share(), &mut rng);

    Ok((nonces, commitments))
}

/// Round 2: Sign with nonces and other parties' commitments
pub fn sign_with_commitments(
    config: &FrostNode,
    message: &[u8],
    nonces: &frost::round1::SigningNonces,
    all_commitments: BTreeMap<frost::Identifier, frost::round1::SigningCommitments>,
) -> Result<frost::round2::SignatureShare> {
    // Create signing package
    let signing_package = frost::SigningPackage::new(all_commitments, message);

    // Generate signature share
    let signature_share = frost::round2::sign(&signing_package, nonces, &config.key_package)?;

    Ok(signature_share)
}

/// Aggregate signature shares into final signature
pub fn aggregate_signature(
    config: &FrostNode,
    message: &[u8],
    commitments: BTreeMap<frost::Identifier, frost::round1::SigningCommitments>,
    signature_shares: BTreeMap<frost::Identifier, frost::round2::SignatureShare>,
) -> Result<frost::Signature> {
    let signing_package = frost::SigningPackage::new(commitments, message);

    let group_signature =
        frost::aggregate(&signing_package, &signature_shares, &config.pubkey_package)?;

    Ok(group_signature)
}

/// Verify a signature
pub fn verify_signature(
    config: &FrostNode,
    message: &[u8],
    signature: &frost::Signature,
) -> Result<bool> {
    Ok(config
        .pubkey_package
        .verifying_key()
        .verify(message, signature)
        .is_ok())
}
