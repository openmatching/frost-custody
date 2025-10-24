/// Secp256k1 ECDSA curve type information for storage routing  
use super::CurveOperations;

pub struct Secp256k1EcdsaOperations;

impl CurveOperations for Secp256k1EcdsaOperations {
    type KeyPackage = frost_secp256k1::keys::KeyPackage;
    type PublicKeyPackage = frost_secp256k1::keys::PublicKeyPackage;
}
