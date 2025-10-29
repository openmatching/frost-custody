use std::sync::{Arc, RwLock};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use bitcoin::hashes::{sha256, Hash};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

/// Trait for providing deterministic RNG from HSM-backed key via PKCS#11
///
/// PKCS#11 is the industry standard API for HSMs and crypto tokens.
/// This abstraction works with ANY PKCS#11-compliant device:
///
/// **Development/Testing:**
/// - SoftHSM (free, software-based)
///
/// **USB Security Keys:**
/// - YubiKey 5 Series
/// - Nitrokey HSM
/// - OnlyKey
///
/// **Enterprise HSMs:**
/// - Thales Luna
/// - Utimaco SecurityServer
/// - Gemalto SafeNet
///
/// **Cloud HSMs:**
/// - AWS CloudHSM
/// - Azure Key Vault with HSM
/// - Google Cloud HSM
///
/// Just change the `pkcs11_library` path in config to switch devices.
/// No code changes needed - same interface for all providers.
pub trait MasterKeyProvider: Send + Sync {
    /// Derive deterministic RNG for a given passphrase
    ///
    /// Same passphrase MUST always produce same RNG seed (for DKG recovery)
    fn derive_rng(&self, passphrase: &str, curve_prefix: &str) -> Result<ChaCha20Rng>;

    /// Get a human-readable description of this provider
    fn description(&self) -> String;

    /// Unlock HSM with PIN
    ///
    /// Returns Ok(true) if unlock successful
    /// Returns Ok(false) if already unlocked
    /// Returns Err if unlock failed
    fn unlock(&self, pin: &str) -> Result<bool>;

    /// Check if HSM is locked
    fn is_locked(&self) -> bool;

    /// Lock the HSM (clear PIN from memory)
    fn lock(&self);

    /// Derive encryption key for RocksDB storage
    ///
    /// This is used to encrypt key shares before storing in RocksDB.
    /// HSM signs passphrase to create deterministic encryption key.
    fn derive_storage_key(&self, passphrase: &str) -> Result<[u8; 32]> {
        // Derive deterministic key for this passphrase
        let mut rng = self.derive_rng(passphrase, "storage-encryption")?;
        let mut key = [0u8; 32];
        use rand::RngCore;
        rng.fill_bytes(&mut key);
        Ok(key)
    }

    /// Encrypt data for storage (AES-256-GCM)
    fn encrypt_storage(&self, passphrase: &str, plaintext: &[u8]) -> Result<Vec<u8>> {
        let key = self.derive_storage_key(passphrase)?;
        let cipher = Aes256Gcm::new(&key.into());

        // Derive deterministic nonce from passphrase (96 bits)
        let nonce_hash = sha256::Hash::hash(format!("nonce:{}", passphrase).as_bytes());
        let mut nonce_arr: [u8; 12] = [0; 12];
        nonce_arr.copy_from_slice(&nonce_hash.as_byte_array()[..12]);
        let nonce: Nonce<U12> = Nonce::from(nonce_arr);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

        Ok(ciphertext)
    }

    /// Decrypt data from storage (AES-256-GCM)
    fn decrypt_storage(&self, passphrase: &str, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let key = self.derive_storage_key(passphrase)?;
        let cipher = Aes256Gcm::new(&key.into());

        // Same deterministic nonce
        let nonce_hash = sha256::Hash::hash(format!("nonce:{}", passphrase).as_bytes());
        let mut nonce_arr: [u8; 12] = [0; 12];
        nonce_arr.copy_from_slice(&nonce_hash.as_byte_array()[..12]);
        let nonce: Nonce<U12> = Nonce::from(nonce_arr);

        let plaintext = cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {:?}", e))?;

        Ok(plaintext)
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

use cryptoki::{
    context::{CInitializeArgs, Pkcs11},
    mechanism::Mechanism,
    object::{Attribute, ObjectHandle},
    session::{Session, UserType},
    types::AuthPin,
};
use sha2::digest::consts::U12;

pub struct Pkcs11KeyProvider {
    pkcs11: Pkcs11,
    slot_id: cryptoki::slot::Slot,
    pin: Arc<RwLock<Option<String>>>, // Interior mutability
    key_label: String,
}

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

        Ok(Self {
            pkcs11,
            slot_id: slot,
            pin: Arc::new(RwLock::new(pin)),
            key_label,
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

impl MasterKeyProvider for Pkcs11KeyProvider {
    fn derive_rng(&self, passphrase: &str, curve_prefix: &str) -> Result<ChaCha20Rng> {
        // Read PIN (blocking read is OK for short critical sections)
        let pin_guard = self
            .pin
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to read PIN: {:?}", e))?;

        // Check if locked
        let pin_str = pin_guard.as_ref().ok_or_else(|| {
            anyhow::anyhow!("HSM is locked. Call /api/hsm/unlock with PIN first.")
        })?;

        // Open session
        let session = self
            .pkcs11
            .open_ro_session(self.slot_id)
            .context("Failed to open PKCS#11 session")?;

        // Login with PIN
        let auth_pin = AuthPin::new(pin_str.clone());
        session
            .login(UserType::User, Some(&auth_pin))
            .context("Failed to login to PKCS#11 token")?;

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

    fn unlock(&self, pin: &str) -> Result<bool> {
        // Read current state
        let pin_guard = self
            .pin
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to read PIN: {:?}", e))?;
        if pin_guard.is_some() {
            drop(pin_guard);
            return Ok(false); // Already unlocked
        }
        drop(pin_guard);

        // Test PIN by trying to open a session and login
        let session = self
            .pkcs11
            .open_ro_session(self.slot_id)
            .context("Failed to open PKCS#11 session")?;

        let auth_pin = AuthPin::new(pin.to_string());
        session
            .login(UserType::User, Some(&auth_pin))
            .context("Failed to login - invalid PIN")?;

        // PIN is valid, store it
        let mut pin_guard = self
            .pin
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to write PIN: {:?}", e))?;
        *pin_guard = Some(pin.to_string());
        drop(pin_guard);

        // Logout and close test session
        let _ = session.logout();
        session.close();

        tracing::info!("🔓 HSM unlocked successfully");
        Ok(true)
    }

    fn is_locked(&self) -> bool {
        self.pin.read().expect("Failed to read PIN").is_none()
    }

    fn lock(&self) {
        let mut pin_guard = self.pin.write().expect("Failed to write PIN");
        *pin_guard = None;
        drop(pin_guard);
        tracing::info!("🔒 HSM locked (PIN cleared from memory)");
    }
}

// ============================================================================
// Factory function
// ============================================================================

#[derive(Debug, Clone, serde::Deserialize)]
pub struct KeyProviderConfig {
    pub pkcs11_library: String,
    pub slot: usize,
    pub pin: Option<String>,
    pub key_label: String,
}

impl KeyProviderConfig {
    pub fn create_provider(&self) -> Result<Box<dyn MasterKeyProvider + 'static>> {
        let provider = Pkcs11KeyProvider::new(
            &self.pkcs11_library,
            self.slot,
            self.pin.clone(),
            self.key_label.clone(),
        )?;
        tracing::info!("Using PKCS#11 HSM key provider: {}", provider.description());
        Ok(Box::new(provider))
    }
}

#[cfg(test)]
mod tests {
    // PKCS#11/HSM key provider tests require actual HSM setup
    // Run integration tests with: cargo xtask test-dkg
    // This will use SoftHSM for testing
}
