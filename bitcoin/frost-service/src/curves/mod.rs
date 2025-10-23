/// Curve abstraction for multi-chain FROST signing
///
/// This module provides a unified interface for different elliptic curves,
/// allowing the same FROST infrastructure to support multiple blockchains.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod ed25519;
pub mod secp256k1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveType {
    Secp256k1,
    Ed25519,
}

impl CurveType {
    pub fn column_family_prefix(&self) -> &'static str {
        match self {
            CurveType::Secp256k1 => "secp256k1",
            CurveType::Ed25519 => "ed25519",
        }
    }
}

/// Trait for curve-specific operations
///
/// This abstracts the differences between secp256k1 (Bitcoin/Ethereum)
/// and Ed25519 (Solana) while keeping the FROST protocol identical.
pub trait CurveOperations: Send + Sync {
    /// Key package type for this curve
    type KeyPackage: Serialize + for<'de> Deserialize<'de> + Debug + Clone;

    /// Public key package type for this curve
    type PublicKeyPackage: Serialize + for<'de> Deserialize<'de> + Debug + Clone;

    /// Signing nonces type
    type SigningNonces: Serialize + for<'de> Deserialize<'de> + Debug + Clone;

    /// Signing commitments type
    type SigningCommitments: Serialize + for<'de> Deserialize<'de> + Debug + Clone;

    /// Signature share type
    type SignatureShare: Serialize + for<'de> Deserialize<'de> + Debug + Clone;

    /// Final signature type
    type Signature: Serialize + for<'de> Deserialize<'de> + Debug + Clone;

    /// DKG Round 1 secret package
    type Round1Secret: Serialize + for<'de> Deserialize<'de> + Debug + Clone;

    /// DKG Round 1 package
    type Round1Package: Serialize + for<'de> Deserialize<'de> + Debug + Clone;

    /// DKG Round 2 secret package
    type Round2Secret: Serialize + for<'de> Deserialize<'de> + Debug + Clone;

    /// DKG Round 2 package
    type Round2Package: Serialize + for<'de> Deserialize<'de> + Debug + Clone;

    /// Identifier type
    type Identifier: Serialize + for<'de> Deserialize<'de> + Debug + Clone;

    /// Run DKG Round 1
    fn dkg_round1(
        &self,
        master_seed: &[u8],
        passphrase: &str,
        node_index: u16,
        max_signers: u16,
        min_signers: u16,
    ) -> Result<(Self::Round1Secret, Self::Round1Package)>;

    /// Run DKG Round 2
    fn dkg_round2(
        &self,
        round1_secret: Self::Round1Secret,
        round1_packages: &[Self::Round1Package],
    ) -> Result<(Self::Round2Secret, Self::Round2Package)>;

    /// Finalize DKG (Round 3)
    fn dkg_finalize(
        &self,
        round2_secret: Self::Round2Secret,
        round1_packages: &[Self::Round1Package],
        round2_packages: &[Self::Round2Package],
    ) -> Result<(Self::KeyPackage, Self::PublicKeyPackage)>;

    /// Create signing commitments (round 1 of signing)
    fn sign_round1(
        &self,
        key_package: &Self::KeyPackage,
        message: &[u8],
    ) -> Result<(Self::SigningNonces, Self::SigningCommitments)>;

    /// Create signature share (round 2 of signing)
    fn sign_round2(
        &self,
        key_package: &Self::KeyPackage,
        nonces: Self::SigningNonces,
        message: &[u8],
        commitments: &[Self::SigningCommitments],
    ) -> Result<Self::SignatureShare>;

    /// Aggregate signature shares into final signature
    fn aggregate_signature(
        &self,
        pubkey_package: &Self::PublicKeyPackage,
        message: &[u8],
        commitments: &[Self::SigningCommitments],
        shares: &[Self::SignatureShare],
    ) -> Result<Self::Signature>;

    /// Verify signature
    fn verify_signature(
        &self,
        pubkey_package: &Self::PublicKeyPackage,
        message: &[u8],
        signature: &Self::Signature,
    ) -> Result<bool>;

    /// Serialize identifier to hex
    fn identifier_to_hex(&self, id: &Self::Identifier) -> String;

    /// Deserialize identifier from hex
    fn identifier_from_hex(&self, hex: &str) -> Result<Self::Identifier>;
}
