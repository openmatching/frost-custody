// Complete example: Build and sign Ethereum transaction using FROST threshold signatures
//
// Prerequisites:
//   docker-compose up frost-node0 frost-node1 frost-node2 address-aggregator signing-aggregator
//
// Run with: cargo run --example sign_eth_frost

use anyhow::Result;
use ethers_core::types::{
    transaction::eip2718::TypedTransaction, Address, Signature, TransactionRequest, U256,
};
use ethers_core::utils::{keccak256, rlp};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== FROST Ethereum Transaction Signing Example ===\n");

    let address_aggregator = "http://127.0.0.1:9000";
    let signing_aggregator = "http://127.0.0.1:8000";

    // Step 1: Generate Ethereum address (includes public key!)
    println!("Step 1: Generate FROST Ethereum address\n");

    let passphrase = "eth-wallet-001".to_string();

    let (eth_address_str, public_key_hex) =
        generate_address_with_pubkey(address_aggregator, &passphrase).await?;
    let eth_address: Address = eth_address_str.parse()?;

    println!("  Ethereum Address: {}", eth_address);
    println!("  Public Key: {}...", &public_key_hex[..16]);
    println!("  Curve: secp256k1 (ECDSA)\n");

    // Step 2: Build Ethereum transaction using ethers
    println!("Step 2: Build Ethereum Transaction\n");

    let to_hex = "742d35cc6634c0532925a3b844bc9e7595f0bebb";
    let to_bytes = hex::decode(to_hex)?;
    let to_address = Address::from_slice(&to_bytes);

    let tx = TransactionRequest::new()
        .from(eth_address)
        .to(to_address)
        .value(U256::from(1_000_000_000_000_000_000u128)) // 1 ETH
        .gas(21_000u64)
        .gas_price(20_000_000_000u64) // 20 Gwei
        .nonce(42u64)
        .chain_id(1u64); // Mainnet

    println!("  From:     {}", eth_address);
    println!("  To:       {}", to_address);
    println!("  Value:    1 ETH");
    println!("  Gas:      21000 @ 20 Gwei");
    println!("  Chain ID: 1 (Mainnet)\n");

    // Step 3: Calculate EIP-155 sighash
    println!("Step 3: Calculate EIP-155 transaction hash\n");

    let sighash = calculate_eip155_sighash(&tx)?;
    println!("  Sighash: 0x{}\n", hex::encode(&sighash));

    // Step 4: Sign with FROST ECDSA
    println!("Step 4: Sign with FROST ECDSA (curve: secp256k1)\n");

    let signature_hex =
        sign_message_via_aggregator(signing_aggregator, &hex::encode(&sighash), &passphrase)
            .await?;

    println!("  ✅ FROST ECDSA signature generated");
    println!("  ✅ Signature length: {} bytes\n", signature_hex.len() / 2);

    // Step 5: Build signed transaction
    println!("Step 5: Build signed Ethereum transaction\n");

    let sig_bytes = hex::decode(&signature_hex)?;

    // Parse ECDSA signature: [r (32), s (32), recovery_id (1)]
    let r = U256::from_big_endian(&sig_bytes[0..32]);
    let s = U256::from_big_endian(&sig_bytes[32..64]);
    let recovery_id = sig_bytes[64] as u64;

    // EIP-155: v = chain_id * 2 + 35 + recovery_id
    let v = 1 * 2 + 35 + recovery_id;

    let eth_signature = Signature { r, s, v };

    // Build final signed transaction with ethers
    let typed_tx: TypedTransaction = tx.into();
    let signed_rlp = typed_tx.rlp_signed(&eth_signature);

    println!(
        "  Signed TX (RLP): 0x{}...",
        hex::encode(&signed_rlp)[..32].to_string()
    );
    println!("  Transaction size: {} bytes", signed_rlp.len());
    println!("  Ready to broadcast to Ethereum network\n");

    // Step 6: Verify signature (server-side method)
    println!("Step 6: Server-side signature verification\n");
    println!("  For production custody, verify signature using:");
    println!("  • Public key: {}", &public_key_hex[..40]);
    println!("  • Message hash: 0x{}", hex::encode(&sighash));
    println!("  • Use secp256k1.verify(hash, signature, pubkey)");
    println!("  • No ecrecover() needed (you know the signer!)\n");

    println!("═══════════════════════════════════════════════════");
    println!("✅ Complete FROST Ethereum transaction signing");
    println!("✅ ECDSA signature verified by FROST aggregator");
    println!("✅ Transaction ready for broadcast");
    println!("✅ Real ethers-core types and encoding");
    println!("═══════════════════════════════════════════════════");

    Ok(())
}

/// Generate Ethereum address and get public key
async fn generate_address_with_pubkey(
    aggregator: &str,
    passphrase: &str,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Req {
        chain: String,
        passphrase: String,
    }

    #[derive(Deserialize)]
    struct Resp {
        address: String,
        public_key: String,
    }

    let resp = reqwest::Client::new()
        .post(format!("{}/api/address/generate", aggregator))
        .json(&Req {
            chain: "ethereum".to_string(),
            passphrase: passphrase.to_string(),
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Generate failed: {}", resp.text().await?);
    }

    let data = resp.json::<Resp>().await?;
    Ok((data.address, data.public_key))
}

/// Sign message via signing aggregator (ECDSA)
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
            curve: "secp256k1".to_string(), // ECDSA for Ethereum
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

/// Calculate EIP-155 sighash using ethers-core
fn calculate_eip155_sighash(tx: &TransactionRequest) -> Result<[u8; 32]> {
    // Build EIP-155 signing hash: Keccak256(RLP([nonce, gasPrice, gasLimit, to, value, data, chainId, 0, 0]))
    let mut stream = rlp::RlpStream::new();
    stream.begin_list(9);

    stream.append(&tx.nonce.unwrap_or(U256::zero()));
    stream.append(&tx.gas_price.unwrap_or(U256::zero()));
    stream.append(&tx.gas.unwrap_or(U256::zero()));

    if let Some(to) = tx.to.as_ref() {
        stream.append(to);
    } else {
        stream.append(&"");
    }

    stream.append(&tx.value.unwrap_or(U256::zero()));
    stream.append(&tx.data.as_ref().map(|d| d.as_ref()).unwrap_or(&[]));

    // EIP-155: append chain_id, 0, 0
    let chain_id_u64 = tx.chain_id.map(|id| id.as_u64()).unwrap_or(1u64);
    stream.append(&chain_id_u64);
    stream.append(&0u8);
    stream.append(&0u8);

    let encoded = stream.out();
    Ok(keccak256(encoded))
}
