/// Ed25519 curve type information for storage routing
use super::CurveOperations;

pub struct Ed25519Operations;

impl CurveOperations for Ed25519Operations {
    type KeyPackage = frost_ed25519::keys::KeyPackage;
    type PublicKeyPackage = frost_ed25519::keys::PublicKeyPackage;
}
