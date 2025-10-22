// Example: Sign Bitcoin transaction using FROST aggregator
// Run with: cargo run --example frost_aggregator_example

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SignRequest {
    message: String,
}

#[derive(Deserialize, Debug)]
struct SignResponse {
    signature: String,
    verified: bool,
    signers_used: usize,
}

#[derive(Deserialize, Debug)]
struct AddressResponse {
    passphrase: String,
    address: String,
    script_type: String,
}

#[derive(Deserialize, Debug)]
struct NodeHealth {
    url: String,
    healthy: bool,
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
struct HealthResponse {
    status: String,
    signer_nodes_total: usize,
    signer_nodes_healthy: usize,
    threshold: usize,
    nodes: Vec<NodeHealth>,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== FROST Aggregator Example ===\n");

    let aggregator_url = "http://127.0.0.1:6000";
    let client = reqwest::Client::new();

    // 1. Health check
    println!("Step 1: Check aggregator health");
    let health: HealthResponse = client
        .get(format!("{}/health", aggregator_url))
        .send()
        .await?
        .json()
        .await?;

    println!("  Status: {}", health.status);
    println!(
        "  Nodes: {} healthy / {} total (need {})",
        health.signer_nodes_healthy, health.signer_nodes_total, health.threshold
    );
    for node in &health.nodes {
        if node.healthy {
            println!("    ✅ {}", node.url);
        } else {
            println!("    ❌ {}: {:?}", node.url, node.error);
        }
    }
    println!();

    // 2. Get Taproot address
    println!("Step 2: Get Taproot address");
    let passphrase = "550e8400-e29b-41d4-a716-446655440000";

    let addr: AddressResponse = client
        .get(format!("{}/api/address", aggregator_url))
        .query(&[("passphrase", passphrase)])
        .send()
        .await?
        .json()
        .await?;

    println!("  Passphrase: {}", addr.passphrase);
    println!("  Address: {}", addr.address);
    println!("  Type: {}\n", addr.script_type);

    // 3. Sign message (e.g., Bitcoin sighash)
    println!("Step 3: Sign message with FROST");
    let message = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    println!("  Message: {}...", &message[..32]);
    println!("  Calling aggregator (orchestrates 3-round FROST)...\n");

    let sign_resp: SignResponse = client
        .post(format!("{}/api/sign", aggregator_url))
        .json(&SignRequest {
            message: message.to_string(),
        })
        .send()
        .await?
        .json()
        .await?;

    println!("  ✅ Signature: {}...", &sign_resp.signature[..32]);
    println!("  ✅ Verified: {}", sign_resp.verified);
    println!("  ✅ Signers used: {}\n", sign_resp.signers_used);

    println!("=== Summary ===");
    println!("✅ Aggregator handled all FROST complexity");
    println!("✅ CEX only made simple HTTP calls");
    println!("✅ Signer nodes remain isolated");
    println!("✅ Single Schnorr signature (56% cheaper than multisig)");

    Ok(())
}
