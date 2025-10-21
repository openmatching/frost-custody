// Example client showing how CEX backend would interact with signer nodes
// Run with: cargo run --example client

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SignRequest {
    psbt: String,
    derivation_ids: Vec<u64>,
}

#[derive(Deserialize, Debug)]
struct SignResponse {
    psbt: String,
    signed_count: usize,
    node_index: u8,
}

#[derive(Deserialize, Debug)]
struct AddressResponse {
    user_id: u64,
    address: String,
    script_type: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Example 1: Get deposit address for user 123
    println!("=== Example 1: Get Deposit Address ===");
    let resp = reqwest::get("http://localhost:3000/api/address?id=123")
        .await?
        .json::<AddressResponse>()
        .await?;
    
    println!("User {}: {}", resp.user_id, resp.address);
    println!("Script type: {}\n", resp.script_type);

    // Example 2: Sign PSBT (pseudo-code, you'd need a real PSBT)
    println!("=== Example 2: Sign PSBT ===");
    let example_psbt = "cHNidP8BAH..."; // Your base64-encoded PSBT
    let derivation_ids = vec![123, 456, 789];

    // Sign with node 0
    let client = reqwest::Client::new();
    let resp = client
        .post("http://localhost:3000/api/sign")
        .json(&SignRequest {
            psbt: example_psbt.to_string(),
            derivation_ids: derivation_ids.clone(),
        })
        .send()
        .await?;
    
    if resp.status().is_success() {
        let sign_resp = resp.json::<SignResponse>().await?;
        println!("Node {} signed {} inputs", sign_resp.node_index, sign_resp.signed_count);
        
        // Sign with node 1 (to get 2-of-3)
        let resp = client
            .post("http://localhost:3001/api/sign")
            .json(&SignRequest {
                psbt: sign_resp.psbt,
                derivation_ids,
            })
            .send()
            .await?
            .json::<SignResponse>()
            .await?;
        
        println!("Node {} signed {} inputs", resp.node_index, resp.signed_count);
        println!("PSBT now has 2-of-3 signatures, ready to finalize!");
    } else {
        println!("Error: {}", resp.text().await?);
    }

    Ok(())
}

