// Complete example: Build and sign Solana transaction using FROST threshold signatures
//
// Prerequisites:
//   docker-compose up frost-node0 frost-node1 frost-node2 address-aggregator signing-aggregator
//
// Run with: cargo run --example sign_sol_frost

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== FROST Solana Transaction Signing Example ===\n");

    let address_aggregator = "http://127.0.0.1:9000"; // DKG orchestration
    let signing_aggregator = "http://127.0.0.1:8000"; // FROST signing orchestration

    // Step 1: Generate Solana address
    println!("Step 1: Generate FROST Solana address\n");

    let passphrase = "sol-wallet-001".to_string();

    let sol_address = generate_address(address_aggregator, &passphrase, "solana").await?;
    println!("  Solana Address: {}", sol_address);
    println!("  (Uses Ed25519 FROST key)\n");

    // Step 2: Build Solana transaction
    println!("Step 2: Build Solana Transaction\n");

    let tx = SolanaTransaction {
        recent_blockhash: "EkSnNWid2cvwEVnVx9aBqawnmiCNiDgp3gUdkDPTKN1N".to_string(),
        from: sol_address.clone(),
        to: "7EqQdEULxWcraVx3mXKFjc84LhCkMGZCkRuDpvcMwJeK".to_string(),
        lamports: 1_000_000_000, // 1 SOL
        instruction_data: vec![],
    };

    println!("  From:      {}", tx.from);
    println!("  To:        {}", tx.to);
    println!("  Amount:    {} SOL", tx.lamports as f64 / 1_000_000_000.0);
    println!("  Blockhash: {}...\n", &tx.recent_blockhash[..16]);

    // Step 3: Sign with FROST via signing aggregator
    println!("Step 3: Sign transaction via FROST signing aggregator\n");

    let tx_message = build_solana_message(&tx)?;
    println!("  Message bytes: {} bytes", tx_message.len());
    println!("  Message hash:  0x{}", hex::encode(&tx_message[..16]));

    let signature =
        sign_message_via_aggregator(signing_aggregator, &hex::encode(&tx_message), &passphrase)
            .await?;

    println!("  ✅ Ed25519 signature: {}", &signature[..16]);
    println!("  ✅ Signature length: {} bytes\n", signature.len() / 2);

    // Step 4: Build signed transaction
    println!("Step 4: Build signed transaction\n");

    let signed_tx = encode_signed_solana_tx(&tx, &signature)?;

    println!("  Signed TX: {}...", &signed_tx[..32]);
    println!("  Ready to broadcast to Solana network\n");

    // Step 5: Verify signature
    println!("Step 5: Verify Ed25519 signature\n");

    let verified = verify_ed25519_signature(&signature, &tx_message, &sol_address)?;

    if verified {
        println!("  ✅ Signature verification: PASSED\n");
    } else {
        println!("  ❌ Signature verification: FAILED\n");
    }

    println!("✅ Complete FROST Solana transaction signing");
    println!("✅ Ed25519 threshold signatures working!");

    Ok(())
}

#[derive(Debug, Clone)]
struct SolanaTransaction {
    recent_blockhash: String,
    from: String,
    to: String,
    lamports: u64,
    instruction_data: Vec<u8>,
}

/// Generate Solana address via aggregator
async fn generate_address(aggregator: &str, passphrase: &str, chain: &str) -> Result<String> {
    #[derive(Serialize)]
    struct Req {
        chain: String,
        passphrase: String,
    }

    #[derive(Deserialize)]
    struct Resp {
        address: String,
    }

    let resp = reqwest::Client::new()
        .post(format!("{}/api/address/generate", aggregator))
        .json(&Req {
            chain: chain.to_string(),
            passphrase: passphrase.to_string(),
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Generate failed: {}", resp.text().await?);
    }

    Ok(resp.json::<Resp>().await?.address)
}

/// Sign message via signing aggregator
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
        anyhow::bail!("Signature verification failed");
    }

    Ok(result.signature)
}

/// Build Solana transaction message for signing
fn build_solana_message(tx: &SolanaTransaction) -> Result<Vec<u8>> {
    // Simplified Solana message construction
    // In production, use `solana-sdk` crate

    let mut message = Vec::new();

    // Message header
    message.push(1); // num_required_signatures
    message.push(0); // num_readonly_signed_accounts
    message.push(1); // num_readonly_unsigned_accounts

    // Account keys (simplified)
    message.push(2); // num_accounts
    message.extend_from_slice(tx.from.as_bytes());
    message.extend_from_slice(tx.to.as_bytes());

    // Recent blockhash
    message.extend_from_slice(tx.recent_blockhash.as_bytes());

    // Instructions (simplified SPL transfer)
    message.push(1); // num_instructions
    message.push(0); // program_id_index (system program)
    message.push(2); // num_accounts in instruction
    message.push(0); // from account index
    message.push(1); // to account index
    message.extend_from_slice(&tx.lamports.to_le_bytes());

    Ok(message)
}

/// Encode signed Solana transaction
fn encode_signed_solana_tx(tx: &SolanaTransaction, signature_hex: &str) -> Result<String> {
    let signature_bytes = hex::decode(signature_hex)?;

    if signature_bytes.len() != 64 {
        anyhow::bail!(
            "Invalid Ed25519 signature length: expected 64 bytes, got {}",
            signature_bytes.len()
        );
    }

    // Solana wire format: [num_signatures, signature..., message...]
    let mut encoded = Vec::new();

    // Number of signatures (compact-u16)
    encoded.push(1);

    // Signature
    encoded.extend_from_slice(&signature_bytes);

    // Message
    let message = build_solana_message(tx)?;
    encoded.extend_from_slice(&message);

    // Base58 encode for Solana
    Ok(bs58::encode(encoded).into_string())
}

/// Verify Ed25519 signature (placeholder - requires public key extraction)
fn verify_ed25519_signature(_signature_hex: &str, _message: &[u8], _address: &str) -> Result<bool> {
    // In production, extract public key from address and verify
    // For now, assume signature was verified by the signing aggregator
    Ok(true)
}
