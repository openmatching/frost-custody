/// Secp256k1-tr (Taproot) curve type information for storage routing
use super::CurveOperations;

pub struct Secp256k1Operations;

impl CurveOperations for Secp256k1Operations {
    type KeyPackage = frost_secp256k1_tr::keys::KeyPackage;
    type PublicKeyPackage = frost_secp256k1_tr::keys::PublicKeyPackage;
}
