use frost_secp256k1_tr as frost;
use std::collections::HashMap;
use std::sync::Mutex;

/// Temporary storage for DKG in-progress state
/// Holds secret packages during DKG rounds
pub struct DkgState {
    round1_secrets: Mutex<HashMap<String, frost::keys::dkg::round1::SecretPackage>>,
    round2_secrets: Mutex<HashMap<String, frost::keys::dkg::round2::SecretPackage>>,
    // Generic storage for other curves (Ed25519, etc.) as serialized bytes
    pub generic_secrets: Mutex<HashMap<String, Vec<u8>>>,
}

impl DkgState {
    pub fn new() -> Self {
        Self {
            round1_secrets: Mutex::new(HashMap::new()),
            round2_secrets: Mutex::new(HashMap::new()),
            generic_secrets: Mutex::new(HashMap::new()),
        }
    }

    pub fn store_round1_secret(
        &self,
        passphrase: &str,
        secret: frost::keys::dkg::round1::SecretPackage,
    ) {
        self.round1_secrets
            .lock()
            .unwrap()
            .insert(passphrase.to_string(), secret);
    }

    pub fn get_round1_secret(
        &self,
        passphrase: &str,
    ) -> Option<frost::keys::dkg::round1::SecretPackage> {
        self.round1_secrets.lock().unwrap().get(passphrase).cloned()
    }

    #[allow(dead_code)]
    pub fn remove_round1_secret(
        &self,
        passphrase: &str,
    ) -> Option<frost::keys::dkg::round1::SecretPackage> {
        self.round1_secrets.lock().unwrap().remove(passphrase)
    }

    pub fn store_round2_secret(
        &self,
        passphrase: &str,
        secret: frost::keys::dkg::round2::SecretPackage,
    ) {
        self.round2_secrets
            .lock()
            .unwrap()
            .insert(passphrase.to_string(), secret);
    }

    #[allow(dead_code)]
    pub fn get_round2_secret(
        &self,
        passphrase: &str,
    ) -> Option<frost::keys::dkg::round2::SecretPackage> {
        self.round2_secrets.lock().unwrap().get(passphrase).cloned()
    }

    pub fn remove_round2_secret(
        &self,
        passphrase: &str,
    ) -> Option<frost::keys::dkg::round2::SecretPackage> {
        self.round2_secrets.lock().unwrap().remove(passphrase)
    }
}
