use std::sync::{Arc, RwLock};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
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
#[async_trait]
pub trait MasterKeyProvider: Send + Sync {
    /// Derive deterministic RNG for a given passphrase (async for cloud providers)
    ///
    /// Same passphrase MUST always produce same RNG seed (for DKG recovery)
    async fn derive_rng(&self, passphrase: &str, curve_prefix: &str) -> Result<ChaCha20Rng>;

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
    async fn derive_storage_key(&self, passphrase: &str) -> Result<[u8; 32]>;

    /// Encrypt data for storage (AES-256-GCM)
    async fn encrypt_storage(&self, passphrase: &str, plaintext: &[u8]) -> Result<Vec<u8>> {
        let key = self.derive_storage_key(passphrase).await?;
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
    async fn decrypt_storage(&self, passphrase: &str, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let key = self.derive_storage_key(passphrase).await?;
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
    /// * `key_label` - Label of the AES-256 key for HMAC (e.g., "frost-hmac-key-node0")
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
            Attribute::Class(cryptoki::object::ObjectClass::SECRET_KEY), // AES key for HMAC
        ];

        let objects = session
            .find_objects(&template)
            .context("Failed to find HMAC key objects")?;

        objects.first().copied().context(format!(
            "HMAC key with label '{}' not found",
            self.key_label
        ))
    }
}

#[async_trait]
impl MasterKeyProvider for Pkcs11KeyProvider {
    async fn derive_rng(&self, passphrase: &str, curve_prefix: &str) -> Result<ChaCha20Rng> {
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

        // Find the AES key
        let aes_key = self.find_key(&session)?;

        // Prepare input: hash(curve_prefix:passphrase) to get exactly 32 bytes for AES
        let mut input_data = Vec::new();
        if !curve_prefix.is_empty() {
            input_data.extend_from_slice(curve_prefix.as_bytes());
            input_data.extend_from_slice(b":");
        }
        input_data.extend_from_slice(passphrase.as_bytes());
        let input_hash = sha256::Hash::hash(&input_data);

        // Encrypt with AES-256-ECB using HSM (DETERMINISTIC and SECURE)
        // AES-ECB is deterministic: same key + same plaintext = same ciphertext
        // This is SAFE for key derivation (not bulk encryption)
        // Attacker CANNOT compute this without HSM AES key
        let mechanism = Mechanism::AesEcb;
        let ciphertext = session
            .encrypt(&mechanism, aes_key, input_hash.as_byte_array())
            .context("Failed to AES-encrypt with HSM key")?;

        // Use ciphertext as RNG seed (deterministic and secure)
        let mut rng_seed = [0u8; 32];
        rng_seed.copy_from_slice(&ciphertext[..32]);

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

    async fn derive_storage_key(&self, passphrase: &str) -> Result<[u8; 32]> {
        let mut rng = self.derive_rng(passphrase, "storage-encryption").await?;
        let mut key = [0u8; 32];
        use rand::RngCore;
        rng.fill_bytes(&mut key);
        Ok(key)
    }
}

// ============================================================================
// AWS KMS Provider
// ============================================================================

#[cfg(feature = "aws-kms")]
pub struct AwsKmsKeyProvider {
    client: aws_sdk_kms::Client,
    key_id: String,
}

#[cfg(feature = "aws-kms")]
impl AwsKmsKeyProvider {
    /// Create new AWS KMS key provider
    pub async fn new(key_id: String) -> Result<Self> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .load()
            .await;
        let client = aws_sdk_kms::Client::new(&config);

        // Test access
        client
            .describe_key()
            .key_id(&key_id)
            .send()
            .await
            .context("Failed to access AWS KMS key - check IAM permissions")?;

        Ok(Self { client, key_id })
    }

    async fn sign_async(&self, message: &[u8]) -> Result<Vec<u8>> {
        let response = self
            .client
            .sign()
            .key_id(&self.key_id)
            .message(aws_sdk_kms::primitives::Blob::new(message))
            .signing_algorithm(aws_sdk_kms::types::SigningAlgorithmSpec::EcdsaSha256)
            .send()
            .await
            .context("Failed to sign with AWS KMS")?;

        let signature = response
            .signature()
            .context("No signature returned from AWS KMS")?
            .as_ref()
            .to_vec();

        Ok(signature)
    }
}

#[cfg(feature = "aws-kms")]
#[async_trait]
impl MasterKeyProvider for AwsKmsKeyProvider {
    async fn derive_rng(&self, passphrase: &str, curve_prefix: &str) -> Result<ChaCha20Rng> {
        // Prepare message
        let mut message = Vec::new();
        if !curve_prefix.is_empty() {
            message.extend_from_slice(curve_prefix.as_bytes());
            message.extend_from_slice(b":");
        }
        message.extend_from_slice(passphrase.as_bytes());

        // Hash message
        let message_hash = sha256::Hash::hash(&message);

        // Sign with KMS (now properly async)
        let signature = self
            .sign_async(message_hash.as_ref())
            .await
            .context("Failed to derive RNG from AWS KMS")?;

        // Derive deterministic RNG from signature
        let seed_hash = sha256::Hash::hash(&signature);
        let mut seed = [0u8; 32];
        seed.copy_from_slice(seed_hash.as_ref());

        Ok(ChaCha20Rng::from_seed(seed))
    }

    fn description(&self) -> String {
        format!(
            "AWS KMS (key={})",
            &self.key_id[..16.min(self.key_id.len())]
        )
    }

    // AWS KMS doesn't have "unlock/lock" concept - IAM controls access
    fn unlock(&self, _pin: &str) -> Result<bool> {
        Ok(true) // Always "unlocked" (controlled by AWS IAM)
    }

    fn is_locked(&self) -> bool {
        false // AWS IAM controls access, not local lock
    }

    fn lock(&self) {
        // No-op for AWS KMS
    }

    async fn derive_storage_key(&self, passphrase: &str) -> Result<[u8; 32]> {
        let mut rng = self.derive_rng(passphrase, "storage-encryption").await?;
        let mut key = [0u8; 32];
        use rand::RngCore;
        rng.fill_bytes(&mut key);
        Ok(key)
    }
}

// ============================================================================
// Factory function
// ============================================================================

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum KeyProviderConfig {
    #[cfg(feature = "pkcs11")]
    Pkcs11 {
        pkcs11_library: String,
        slot: usize,
        pin: Option<String>,
        key_label: String,
    },
    #[cfg(feature = "aws-kms")]
    #[serde(rename = "aws-kms")]
    AwsKms { key_id: String },
}

impl KeyProviderConfig {
    pub async fn create_provider(&self) -> Result<Box<dyn MasterKeyProvider + 'static>> {
        match self {
            #[cfg(feature = "pkcs11")]
            KeyProviderConfig::Pkcs11 {
                pkcs11_library,
                slot,
                pin,
                key_label,
            } => {
                let provider =
                    Pkcs11KeyProvider::new(pkcs11_library, *slot, pin.clone(), key_label.clone())?;
                tracing::info!("Using PKCS#11 HSM key provider: {}", provider.description());
                Ok(Box::new(provider))
            }
            #[cfg(feature = "aws-kms")]
            KeyProviderConfig::AwsKms { key_id } => {
                let provider = AwsKmsKeyProvider::new(key_id.clone()).await?;
                tracing::info!("Using AWS KMS key provider: {}", provider.description());
                Ok(Box::new(provider))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // PKCS#11/HSM key provider tests require actual HSM setup
    // Run integration tests with: cargo xtask test-dkg
    // This will use SoftHSM for testing
}
