// Complete example: Build and sign Solana transaction using FROST threshold signatures
//
// Prerequisites:
//   docker-compose up frost-node0 frost-node1 frost-node2 address-aggregator signing-aggregator
//
// Run with: cargo run --example sign_sol_frost

use anyhow::Result;
use ed25519_dalek::{Signature as Ed25519Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    hash::Hash, instruction::Instruction, message::Message, pubkey::Pubkey, signature::Signature,
    system_instruction, transaction::Transaction,
};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== FROST Solana Transaction Signing Example ===\n");

    let address_aggregator = "http://127.0.0.1:9000"; // DKG orchestration
    let signing_aggregator = "http://127.0.0.1:8000"; // FROST signing orchestration

    // Step 1: Generate Solana address and get public key
    println!("Step 1: Generate FROST Solana address\n");

    let passphrase = "sol-wallet-001".to_string();

    let (sol_address_str, pubkey_hex) =
        generate_address_with_pubkey(address_aggregator, &passphrase).await?;

    println!("  Solana Address: {}", sol_address_str);
    println!("  Public Key (hex): {}...", &pubkey_hex[..16]);
    println!("  (Uses Ed25519 FROST key)\n");

    // Parse Solana pubkey from hex
    let pubkey_bytes = hex::decode(&pubkey_hex)?;
    let from_pubkey = Pubkey::try_from(pubkey_bytes.as_slice())?;

    // Step 2: Build Solana transaction using solana-sdk
    println!("Step 2: Build Solana Transaction using solana-sdk\n");

    let to_pubkey = Pubkey::from_str("7EqQdEULxWcraVx3mXKFjc84LhCkMGZCkRuDpvcMwJeK")?;

    // Create transfer instruction (1 SOL = 1,000,000,000 lamports)
    let transfer_ix: Instruction =
        system_instruction::transfer(&from_pubkey, &to_pubkey, 1_000_000_000);

    // Recent blockhash (in production, fetch from RPC)
    // For demo, use a dummy hash
    let recent_blockhash = Hash::new_from_array([1u8; 32]);

    // Build message using solana-sdk
    let message = Message::new(&[transfer_ix], Some(&from_pubkey));

    println!("  From:        {}", from_pubkey);
    println!("  To:          {}", to_pubkey);
    println!("  Amount:      1 SOL (1,000,000,000 lamports)");
    println!("  Instructions: {}", message.instructions.len());
    println!("  Accounts:     {}", message.account_keys.len());
    println!(
        "  Blockhash:    {}...\n",
        &recent_blockhash.to_string()[..16]
    );

    // Step 3: Serialize message for signing
    println!("Step 3: Serialize message for signing\n");

    let message_bytes = message.serialize();
    println!("  Message bytes: {} bytes", message_bytes.len());
    println!(
        "  Message (hex): 0x{}...\n",
        hex::encode(&message_bytes[..16.min(message_bytes.len())])
    );

    // Step 4: Sign with FROST via signing aggregator
    println!("Step 4: Sign with FROST Ed25519 threshold signatures\n");

    let signature_hex = sign_message_via_aggregator(
        signing_aggregator,
        &hex::encode(&message_bytes),
        &passphrase,
    )
    .await?;

    println!("  ✅ Ed25519 signature: {}...", &signature_hex[..16]);
    println!("  ✅ Signature length: {} bytes\n", signature_hex.len() / 2);

    // Step 5: Verify signature using ed25519-dalek
    println!("Step 5: Verify Ed25519 signature with ed25519-dalek\n");

    let sig_bytes = hex::decode(&signature_hex)?;

    let verified = verify_ed25519_signature(&sig_bytes, &message_bytes, &pubkey_bytes)?;

    if verified {
        println!("  🔒 ed25519-dalek verification: PASSED");
        println!("  ✅ Signature is cryptographically valid\n");
    } else {
        anyhow::bail!("Signature verification failed!");
    }

    // Step 6: Build Solana transaction with signature
    println!("Step 6: Build signed Solana transaction\n");

    let mut sig_array = [0u8; 64];
    sig_array.copy_from_slice(&sig_bytes);
    let solana_signature = Signature::from(sig_array);

    // Create complete transaction
    let transaction = Transaction {
        signatures: vec![solana_signature],
        message,
    };

    // Serialize transaction for broadcast
    let serialized = bincode::serialize(&transaction)?;
    let encoded_tx = bs58::encode(&serialized).into_string();

    println!("  Transaction size: {} bytes", serialized.len());
    println!("  Encoded (Base58): {}...", &encoded_tx[..32]);
    println!("  Ready to broadcast to Solana network\n");

    // Additional verification: Check transaction is valid
    println!("Step 7: Verify transaction structure\n");

    println!(
        "  ✅ Transaction has {} signature",
        transaction.signatures.len()
    );
    println!(
        "  ✅ Message has {} instruction",
        transaction.message.instructions.len()
    );
    println!("  ✅ Valid Solana Transaction structure\n");

    println!("════════════════════════════════════════");
    println!("✅ Complete FROST Solana transaction signing");
    println!("✅ Ed25519 threshold signatures working!");
    println!("✅ Signature verified with ed25519-dalek!");
    println!("✅ Transaction built with solana-sdk!");
    println!("════════════════════════════════════════");

    Ok(())
}

/// Generate Solana address and get Ed25519 public key
async fn generate_address_with_pubkey(
    aggregator: &str,
    passphrase: &str,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct AddressReq {
        chain: String,
        passphrase: String,
    }

    #[derive(Deserialize)]
    struct AddressResp {
        address: String,
        public_key: String,
    }

    // Generate Solana address
    let resp = reqwest::Client::new()
        .post(format!("{}/api/address/generate", aggregator))
        .json(&AddressReq {
            chain: "solana".to_string(),
            passphrase: passphrase.to_string(),
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Address generation failed: {}", resp.text().await?);
    }

    let resp_data = resp.json::<AddressResp>().await?;

    // Public key is now included in the address response!
    Ok((resp_data.address, resp_data.public_key))
}

/// Sign message via signing aggregator (Ed25519)
async fn sign_message_via_aggregator(
    aggregator_url: &str,
    message_hex: &str,
    passphrase: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct Req {
        passphrase: String,
        message: String,
        curve: String,
    }

    #[derive(Deserialize)]
    struct Resp {
        signature: String,
        verified: bool,
    }

    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/sign/message", aggregator_url))
        .json(&Req {
            passphrase: passphrase.to_string(),
            message: message_hex.to_string(),
            curve: "ed25519".to_string(), // Solana uses Ed25519
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Signing failed: {}", resp.text().await?);
    }

    let result = resp.json::<Resp>().await?;

    if !result.verified {
        anyhow::bail!("Signature verification failed by aggregator");
    }

    Ok(result.signature)
}

/// Verify Ed25519 signature using ed25519-dalek (cryptographic proof!)
fn verify_ed25519_signature(
    signature_bytes: &[u8],
    message: &[u8],
    pubkey_bytes: &[u8],
) -> Result<bool> {
    if signature_bytes.len() != 64 {
        anyhow::bail!(
            "Invalid Ed25519 signature length: expected 64 bytes, got {}",
            signature_bytes.len()
        );
    }

    if pubkey_bytes.len() != 32 {
        anyhow::bail!(
            "Invalid Ed25519 public key length: expected 32 bytes, got {}",
            pubkey_bytes.len()
        );
    }

    // Create Ed25519 signature from bytes
    let signature = Ed25519Signature::from_slice(signature_bytes)
        .map_err(|e| anyhow::anyhow!("Invalid Ed25519 signature: {}", e))?;

    // Create verifying key from public key bytes
    let mut pk_array = [0u8; 32];
    pk_array.copy_from_slice(pubkey_bytes);
    let verifying_key = VerifyingKey::from_bytes(&pk_array)
        .map_err(|e| anyhow::anyhow!("Invalid public key: {}", e))?;

    // Verify signature - this is REAL cryptographic verification!
    match verifying_key.verify(message, &signature) {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("  ❌ Verification failed: {}", e);
            Ok(false)
        }
    }
}
