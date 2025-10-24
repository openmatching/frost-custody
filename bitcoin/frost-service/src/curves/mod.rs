/// Curve abstraction for multi-chain FROST signing
///
/// This module provides minimal type information for curve-specific storage routing.
/// The actual FROST operations are implemented directly in the node API for clarity.
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

pub mod ed25519;
pub mod secp256k1;
pub mod secp256k1_ecdsa;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveType {
    Secp256k1Taproot, // Schnorr signatures for Bitcoin Taproot
    Secp256k1Ecdsa,   // ECDSA signatures for Ethereum/EVM
    Ed25519,          // Ed25519 signatures for Solana
}

/// Minimal trait for curve type information
/// Only defines associated types needed for storage - no methods!
pub trait CurveOperations: Send + Sync {
    type KeyPackage: Serialize + DeserializeOwned + Debug + Clone;
    type PublicKeyPackage: Serialize + DeserializeOwned + Debug + Clone;
}
