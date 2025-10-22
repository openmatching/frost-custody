use anyhow::{Context, Result};
use bitcoin::hashes::{sha256, Hash};

/// Encrypt nonces with key package secret for secure client-side storage
pub fn encrypt_nonces(nonces_json: &[u8], message: &[u8], key_package_hex: &str) -> Result<String> {
    // Use key_package as encryption key
    let key_bytes = hex::decode(key_package_hex)?;
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
pub fn decrypt_nonces(
    encrypted_hex: &str,
    message: &[u8],
    key_package_hex: &str,
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

    // Decrypt
    let key_bytes = hex::decode(key_package_hex)?;
    let key_hash = sha256::Hash::hash(&key_bytes);

    let mut decrypted = Vec::new();
    for (i, &byte) in ciphertext.iter().enumerate() {
        let key_byte = key_hash.as_byte_array()[i % 32];
        decrypted.push(byte ^ key_byte);
    }

    Ok(decrypted)
}
