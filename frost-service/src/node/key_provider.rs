use anyhow::{Context, Result};
use bitcoin::hashes::{sha256, Hash};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

/// Trait for providing deterministic RNG from master key material
///
/// This abstraction allows different backends:
/// - Plaintext seed (simple, for development)
/// - PKCS#11 (industry standard, works with ANY HSM/token)
///
/// PKCS#11 is vendor-agnostic - works with:
/// - USB tokens: YubiKey, Nitrokey, OnlyKey
/// - Enterprise HSM: Thales, Utimaco, Gemalto
/// - Cloud HSM: AWS CloudHSM, Azure Key Vault
/// - Testing: SoftHSM
pub trait MasterKeyProvider: Send + Sync {
    /// Derive deterministic RNG for a given passphrase
    ///
    /// Same passphrase MUST always produce same RNG seed (for DKG recovery)
    fn derive_rng(&self, passphrase: &str, curve_prefix: &str) -> Result<ChaCha20Rng>;

    /// Get a human-readable description of this provider
    fn description(&self) -> String;

    /// Unlock HSM with PIN (for PKCS#11 providers)
    ///
    /// Returns Ok(true) if unlock successful or not needed
    /// Returns Ok(false) if already unlocked
    /// Returns Err if unlock failed
    fn unlock(&mut self, pin: &str) -> Result<bool> {
        let _ = pin; // Unused for plaintext
        Ok(true) // Plaintext doesn't need unlocking
    }

    /// Check if HSM is locked
    fn is_locked(&self) -> bool {
        false // Plaintext is never locked
    }

    /// Lock the HSM (clear PIN from memory)
    fn lock(&mut self) {
        // Plaintext has nothing to lock
    }
}

// ============================================================================
// Plaintext Implementation (current, default)
// ============================================================================

pub struct PlaintextKeyProvider {
    master_seed: Vec<u8>,
}

impl PlaintextKeyProvider {
    pub fn new(master_seed: Vec<u8>) -> Self {
        Self { master_seed }
    }

    pub fn from_hex(hex_seed: &str) -> Result<Self> {
        let seed = hex::decode(hex_seed).context("Failed to decode master seed hex")?;
        Ok(Self::new(seed))
    }
}

impl MasterKeyProvider for PlaintextKeyProvider {
    fn derive_rng(&self, passphrase: &str, curve_prefix: &str) -> Result<ChaCha20Rng> {
        let mut seed_material = self.master_seed.clone();
        if !curve_prefix.is_empty() {
            seed_material.extend_from_slice(curve_prefix.as_bytes());
            seed_material.extend_from_slice(b":");
        }
        seed_material.extend_from_slice(passphrase.as_bytes());

        let seed_hash = sha256::Hash::hash(&seed_material);
        let seed: [u8; 32] = *seed_hash.as_byte_array();

        Ok(ChaCha20Rng::from_seed(seed))
    }

    fn description(&self) -> String {
        format!(
            "Plaintext seed ({}...)",
            hex::encode(&self.master_seed[..4])
        )
    }
}

// ============================================================================
// PKCS#11 / HSM Implementation (Industry Standard, Vendor-Agnostic)
// ============================================================================
//
// PKCS#11 (Cryptoki) is the industry standard API for HSMs and tokens.
// This implementation works with ANY PKCS#11-compliant device:
// - YubiKey, Nitrokey (USB tokens)
// - Thales, Utimaco (enterprise HSM)
// - AWS CloudHSM, Azure Key Vault (cloud)
// - SoftHSM (software testing)
//
// Just change the pkcs11_library path in config to switch devices.
// No code changes needed.
// ============================================================================

#[cfg(feature = "pkcs11")]
use cryptoki::{
    context::{CInitializeArgs, Pkcs11},
    mechanism::Mechanism,
    object::{Attribute, ObjectHandle},
    session::{Session, UserType},
    types::AuthPin,
};

#[cfg(feature = "pkcs11")]
pub struct Pkcs11KeyProvider {
    pkcs11: Pkcs11,
    slot_id: cryptoki::slot::Slot,
    pin: Option<String>, // Optional: can be provided at runtime via unlock API
    key_label: String,
    locked: bool, // Track lock state
}

#[cfg(feature = "pkcs11")]
impl Pkcs11KeyProvider {
    /// Create new PKCS#11 key provider
    ///
    /// Works with ANY PKCS#11-compliant device.
    ///
    /// # Arguments
    /// * `pkcs11_library` - Path to PKCS#11 library (vendor-specific):
    ///   - YubiKey: "/usr/lib/libykcs11.so"
    ///   - SoftHSM: "/usr/lib/softhsm/libsofthsm2.so"
    ///   - Thales: "/opt/nfast/toolkits/pkcs11/libcknfast.so"
    ///   - AWS CloudHSM: "/opt/cloudhsm/lib/libcloudhsm_pkcs11.so"
    /// * `slot_id` - Slot number (usually 0 for first device)
    /// * `pin` - PIN for the token (optional for some HSMs)
    /// * `key_label` - Label of the key to use (e.g., "frost-node-0")
    pub fn new(
        pkcs11_library: &str,
        slot_id: usize,
        pin: Option<String>,
        key_label: String,
    ) -> Result<Self> {
        let pkcs11 = Pkcs11::new(pkcs11_library).context(format!(
            "Failed to load PKCS#11 library: {}",
            pkcs11_library
        ))?;

        pkcs11
            .initialize(CInitializeArgs::OsThreads)
            .context("Failed to initialize PKCS#11")?;

        let slot = pkcs11
            .get_slots_with_token()
            .context("Failed to get PKCS#11 slots")?
            .into_iter()
            .nth(slot_id)
            .context(format!("Slot {} not found", slot_id))?;

        let locked = pin.is_none(); // If no PIN in config, start locked

        Ok(Self {
            pkcs11,
            slot_id: slot,
            pin,
            key_label,
            locked,
        })
    }

    fn find_key(&self, session: &Session) -> Result<ObjectHandle> {
        let label_value = self.key_label.as_bytes().to_vec();
        let template = vec![
            Attribute::Label(label_value),
            Attribute::Class(cryptoki::object::ObjectClass::PRIVATE_KEY),
        ];

        let objects = session
            .find_objects(&template)
            .context("Failed to find key objects")?;

        objects
            .first()
            .copied()
            .context(format!("Key with label '{}' not found", self.key_label))
    }

    fn sign_with_key(&self, session: &Session, key: ObjectHandle, data: &[u8]) -> Result<Vec<u8>> {
        // SoftHSM doesn't support ECDSA-SHA256 combined mechanism
        // So we hash first, then sign with plain ECDSA
        let mechanism = Mechanism::Ecdsa;

        let signature = session
            .sign(&mechanism, key, data)
            .context("Failed to sign with HSM key")?;

        Ok(signature)
    }
}

#[cfg(feature = "pkcs11")]
impl MasterKeyProvider for Pkcs11KeyProvider {
    fn derive_rng(&self, passphrase: &str, curve_prefix: &str) -> Result<ChaCha20Rng> {
        // Check if locked
        if self.locked {
            anyhow::bail!("HSM is locked. Call /api/unlock with PIN first.");
        }

        // Open session
        let session = self
            .pkcs11
            .open_ro_session(self.slot_id)
            .context("Failed to open PKCS#11 session")?;

        // Login if PIN provided
        if let Some(pin) = &self.pin {
            let auth_pin = AuthPin::new(pin.clone());
            session
                .login(UserType::User, Some(&auth_pin))
                .context("Failed to login to PKCS#11 token")?;
        }

        // Find the key
        let key_handle = self.find_key(&session)?;

        // Prepare message to sign: hash(curve_prefix:passphrase)
        let mut message_data = Vec::new();
        if !curve_prefix.is_empty() {
            message_data.extend_from_slice(curve_prefix.as_bytes());
            message_data.extend_from_slice(b":");
        }
        message_data.extend_from_slice(passphrase.as_bytes());
        let message_hash = sha256::Hash::hash(&message_data);

        // Sign with HSM key (deterministic if HSM supports RFC 6979)
        let signature = self.sign_with_key(&session, key_handle, message_hash.as_ref())?;

        // Derive RNG seed from signature
        // Signature is deterministic → same passphrase = same signature = same RNG
        let rng_seed_hash = sha256::Hash::hash(&signature);
        let rng_seed: [u8; 32] = *rng_seed_hash.as_byte_array();

        // Logout and close session
        let _ = session.logout(); // Ignore errors on logout
        session.close();

        Ok(ChaCha20Rng::from_seed(rng_seed))
    }

    fn description(&self) -> String {
        format!(
            "PKCS#11 HSM (slot={}, key={})",
            self.slot_id.id(),
            self.key_label
        )
    }

    fn unlock(&mut self, pin: &str) -> Result<bool> {
        if !self.locked {
            return Ok(false); // Already unlocked
        }

        // Test PIN by trying to open a session and login
        let session = self
            .pkcs11
            .open_ro_session(self.slot_id)
            .context("Failed to open PKCS#11 session")?;

        let auth_pin = AuthPin::new(pin.to_string());
        session
            .login(UserType::User, Some(&auth_pin))
            .context("Failed to login - invalid PIN")?;

        // PIN is valid, store it and unlock
        self.pin = Some(pin.to_string());
        self.locked = false;

        // Logout and close test session
        let _ = session.logout();
        session.close();

        tracing::info!("🔓 HSM unlocked successfully");
        Ok(true)
    }

    fn is_locked(&self) -> bool {
        self.locked
    }

    fn lock(&mut self) {
        self.pin = None;
        self.locked = true;
        tracing::info!("🔒 HSM locked (PIN cleared from memory)");
    }
}

// ============================================================================
// Factory function
// ============================================================================

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum KeyProviderConfig {
    Plaintext {
        master_seed_hex: String,
    },
    #[cfg(feature = "pkcs11")]
    Pkcs11 {
        pkcs11_library: String,
        slot: usize,
        pin: Option<String>,
        key_label: String,
    },
}

impl KeyProviderConfig {
    pub fn create_provider(&self) -> Result<Box<dyn MasterKeyProvider>> {
        match self {
            KeyProviderConfig::Plaintext { master_seed_hex } => {
                let provider = PlaintextKeyProvider::from_hex(master_seed_hex)?;
                tracing::info!("Using plaintext key provider: {}", provider.description());
                Ok(Box::new(provider))
            }
            #[cfg(feature = "pkcs11")]
            KeyProviderConfig::Pkcs11 {
                pkcs11_library,
                slot,
                pin,
                key_label,
            } => {
                let provider =
                    Pkcs11KeyProvider::new(pkcs11_library, *slot, pin.clone(), key_label.clone())?;
                tracing::info!("Using PKCS#11 key provider: {}", provider.description());
                Ok(Box::new(provider))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plaintext_provider() {
        let seed = vec![0u8; 32];
        let provider = PlaintextKeyProvider::new(seed);

        // Same passphrase should produce same RNG
        let rng1 = provider.derive_rng("test-passphrase", "").unwrap();
        let rng2 = provider.derive_rng("test-passphrase", "").unwrap();

        // Can't directly compare RNGs, but we can verify they produce same output
        let mut rng1 = rng1;
        let mut rng2 = rng2;

        use rand::RngCore;
        assert_eq!(rng1.next_u64(), rng2.next_u64());
    }

    #[test]
    fn test_different_passphrases() {
        let seed = vec![0u8; 32];
        let provider = PlaintextKeyProvider::new(seed);

        let rng1 = provider.derive_rng("passphrase1", "").unwrap();
        let rng2 = provider.derive_rng("passphrase2", "").unwrap();

        // Different passphrases should produce different RNGs
        let mut rng1 = rng1;
        let mut rng2 = rng2;

        use rand::RngCore;
        assert_ne!(rng1.next_u64(), rng2.next_u64());
    }

    #[test]
    fn test_curve_prefix() {
        let seed = vec![0u8; 32];
        let provider = PlaintextKeyProvider::new(seed);

        // Same passphrase but different curve should produce different RNG
        let rng1 = provider.derive_rng("test", "secp256k1").unwrap();
        let rng2 = provider.derive_rng("test", "ed25519").unwrap();

        let mut rng1 = rng1;
        let mut rng2 = rng2;

        use rand::RngCore;
        assert_ne!(rng1.next_u64(), rng2.next_u64());
    }
}
