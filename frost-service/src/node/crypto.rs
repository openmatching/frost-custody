use anyhow::{Context, Result};
use bitcoin::hashes::{sha256, Hash};
use super::key_provider::MasterKeyProvider;
use rand::RngCore;

/// Encrypt nonces with deterministic key for secure server-side storage
pub fn encrypt_nonces_with_provider(
    nonces_json: &[u8],
    message: &[u8],
    key_provider: &dyn MasterKeyProvider,
) -> Result<String> {
    // Derive deterministic encryption key from provider
    let mut rng = key_provider.derive_rng("nonce-encryption", "")?;
    let mut key_bytes = [0u8; 32];
    rng.fill_bytes(&mut key_bytes);
    let key_hash = sha256::Hash::hash(&key_bytes);

    // Bind encryption to message (prevents reuse with different message)
    let message_hash = sha256::Hash::hash(message);

    // Simple XOR encryption (for production, use AES-GCM)
    // This is demonstration - in production use proper AEAD
    let mut encrypted = Vec::new();
    encrypted.extend_from_slice(message_hash.as_byte_array());

    for (i, &byte) in nonces_json.iter().enumerate() {
        let key_byte = key_hash.as_byte_array()[i % 32];
        encrypted.push(byte ^ key_byte);
    }

    Ok(hex::encode(encrypted))
}

/// Decrypt nonces and verify message binding
pub fn decrypt_nonces_with_provider(
    encrypted_hex: &str,
    message: &[u8],
    key_provider: &dyn MasterKeyProvider,
) -> Result<Vec<u8>> {
    let encrypted = hex::decode(encrypted_hex).context("Invalid encrypted nonce hex")?;

    if encrypted.len() < 32 {
        anyhow::bail!("Invalid encrypted nonce: too short");
    }

    // Extract message hash
    let stored_message_hash = &encrypted[0..32];
    let ciphertext = &encrypted[32..];

    // Verify message binding
    let message_hash = sha256::Hash::hash(message);
    if stored_message_hash != message_hash.as_byte_array() {
        anyhow::bail!("Message mismatch - nonce was generated for different message");
    }

    // Decrypt with same deterministic key
    let mut rng = key_provider.derive_rng("nonce-encryption", "")?;
    let mut key_bytes = [0u8; 32];
    rng.fill_bytes(&mut key_bytes);
    let key_hash = sha256::Hash::hash(&key_bytes);

    let mut decrypted = Vec::new();
    for (i, &byte) in ciphertext.iter().enumerate() {
        let key_byte = key_hash.as_byte_array()[i % 32];
        decrypted.push(byte ^ key_byte);
    }

    Ok(decrypted)
}
