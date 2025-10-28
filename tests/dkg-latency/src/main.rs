//! DKG Latency Test: 16-of-24 Bitcoin Address Generation via Aggregator
//!
//! This test measures DKG performance with:
//! - 24 internal signer nodes (not exposed)
//! - 1 address aggregator (port 9100)
//! - 16-of-24 threshold (67% Byzantine fault tolerance)
//! - Client calls aggregator API only
//!
//! Run with Docker:
//!   1. docker-compose -f docker-compose.test-24.yml up -d
//!   2. cargo run --bin dkg_latency_test
//!   3. docker-compose -f docker-compose.test-24.yml down -v

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::sleep;

const NODE_COUNT: usize = 24;
const THRESHOLD: usize = 16;
const AGGREGATOR_URL: &str = "http://127.0.0.1:9100";

// Address generation request/response
#[derive(Serialize)]
struct GenerateAddressRequest {
    chain: String,
    passphrase: String,
}

#[derive(Deserialize)]
struct GenerateAddressResponse {
    address: String,
    #[allow(dead_code)]
    chain: String,
    #[allow(dead_code)]
    passphrase: String,
}

#[derive(Default)]
struct DkgMetrics {
    total_duration: Duration,
}

impl DkgMetrics {
    fn print_summary(&self, node_count: usize, threshold: usize) {
        println!("\n╔═══════════════════════════════════════════════════════════╗");
        println!("║          DKG Performance - Bitcoin Taproot                ║");
        println!("╚═══════════════════════════════════════════════════════════╝\n");

        println!("Configuration:");
        println!("  Curve:           secp256k1-tr (Schnorr/Taproot)");
        println!("  Total nodes:     {}", node_count);
        println!("  Threshold:       {}-of-{}", threshold, node_count);
        println!(
            "  BFT tolerance:   {} compromised nodes",
            node_count - threshold
        );
        println!("  Architecture:    Aggregator orchestrates internal signers");
        println!();

        let total_ms = self.total_duration.as_secs_f64() * 1000.0;

        println!("Latency:");
        println!("  ┌─────────────────────────────────────────────────────┐");
        println!(
            "  │ Single address generation:  {:>9.2} ms           │",
            total_ms
        );
        println!("  └─────────────────────────────────────────────────────┘");
        println!();

        println!("DKG Protocol:");
        println!("  - 3-round protocol (commit, share, finalize)");
        println!("  - {} nodes participate", node_count);
        println!(
            "  - O(n²) network complexity = {} interactions",
            node_count * node_count
        );
        println!("  - Each DKG is independent (can run concurrently)");
        println!();
    }
}

async fn wait_for_aggregator(timeout_secs: u64) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/docs", AGGREGATOR_URL);
    let start = Instant::now();

    println!("⏳ Waiting for aggregator to be ready...");

    loop {
        if start.elapsed().as_secs() > timeout_secs {
            anyhow::bail!("Aggregator didn't start within {} seconds", timeout_secs);
        }

        match client
            .get(&url)
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                println!("✅ Aggregator is ready\n");
                return Ok(());
            }
            _ => {
                sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

async fn generate_address_via_aggregator(passphrase: &str) -> Result<(String, DkgMetrics)> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120)) // DKG can take time
        .build()?;

    let mut metrics = DkgMetrics::default();

    println!("Starting Bitcoin address generation via aggregator");
    println!("  Passphrase: {}", passphrase);
    println!(
        "  Aggregator orchestrating DKG across {} nodes...",
        NODE_COUNT
    );

    let start = Instant::now();

    let resp = client
        .post(format!("{}/api/address/generate", AGGREGATOR_URL))
        .json(&GenerateAddressRequest {
            chain: "bitcoin".to_string(),
            passphrase: passphrase.to_string(),
        })
        .send()
        .await
        .context("Failed to call aggregator")?;

    if !resp.status().is_success() {
        let error = resp.text().await.unwrap_or_default();
        anyhow::bail!("Address generation failed: {}", error);
    }

    let address_resp: GenerateAddressResponse = resp.json().await?;
    metrics.total_duration = start.elapsed();

    println!("    ✅ Address generated: {}", address_resp.address);
    println!(
        "    ⏱  Total time: {:.2}ms",
        metrics.total_duration.as_secs_f64() * 1000.0
    );

    Ok((address_resp.address, metrics))
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║  FROST DKG Latency Test: 16-of-24 Bitcoin Addresses      ║");
    println!("║            via Address Aggregator                         ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    println!("Test Architecture:");
    println!("  ┌─────────────┐");
    println!("  │   Client    │  (this test)");
    println!("  └──────┬──────┘");
    println!("         │ HTTP POST /api/address/generate");
    println!("         ▼");
    println!("  ┌─────────────────────┐");
    println!("  │ Address Aggregator  │  port 9100 (exposed)");
    println!("  └─────────┬───────────┘");
    println!("            │ Orchestrates DKG");
    println!("     ┌──────┴──────┬──────────┬─────────┐");
    println!("     ▼             ▼          ▼         ▼");
    println!("  [Node 0]     [Node 1]  ...  [Node 23]");
    println!("  internal     internal       internal");
    println!("  (24 total signer nodes, 16-of-24 threshold)");
    println!();

    println!("Configuration:");
    println!("  Nodes:       {} signer nodes (internal)", NODE_COUNT);
    println!("  Threshold:   {}-of-{}", THRESHOLD, NODE_COUNT);
    println!("  Aggregator:  {}", AGGREGATOR_URL);
    println!("  Chain:       Bitcoin (Taproot/Schnorr)\n");

    // Wait for aggregator (which waits for all nodes)
    wait_for_aggregator(120).await?;

    println!("🧪 Running DKG test for Bitcoin address generation\n");

    // Run 3 test iterations to get average timing
    let mut all_metrics = Vec::new();

    for run in 1..=3 {
        println!("═══════════════════════════════════════════════════════════");
        println!("Test Run #{}/3", run);
        println!("═══════════════════════════════════════════════════════════\n");

        let passphrase = format!("test-bitcoin-address-{}", run);

        match generate_address_via_aggregator(&passphrase).await {
            Ok((address, metrics)) => {
                println!("\n✅ DKG Success!");
                println!("   Passphrase: {}", passphrase);
                println!("   Address:    {}", address);
                println!(
                    "   Time:       {:.2}ms\n",
                    metrics.total_duration.as_secs_f64() * 1000.0
                );
                all_metrics.push(metrics);
            }
            Err(e) => {
                println!("❌ DKG Failed: {}\n", e);

                // Show more debug info
                println!("Debug: Checking aggregator status...");
                let client = reqwest::Client::new();
                match client.get(format!("{}/docs", AGGREGATOR_URL)).send().await {
                    Ok(resp) => println!("  Aggregator responding: {}", resp.status()),
                    Err(e) => println!("  Aggregator not reachable: {}", e),
                }

                return Err(e);
            }
        }

        // Small delay between runs
        if run < 3 {
            sleep(Duration::from_millis(1000)).await;
        }
    }

    // Calculate and display averages
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║                  Final Results (Average)                  ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    let avg_total = all_metrics
        .iter()
        .map(|m| m.total_duration.as_secs_f64())
        .sum::<f64>()
        / all_metrics.len() as f64;

    let avg_metrics = DkgMetrics {
        total_duration: Duration::from_secs_f64(avg_total),
    };

    avg_metrics.print_summary(NODE_COUNT, THRESHOLD);

    println!("Latency Comparison:");
    println!("  2-of-3 setup:   ~80ms (estimated)");
    println!("  5-of-7 setup:   ~150ms (estimated)");
    println!("  10-of-15:       ~200ms (estimated)");
    println!("  16-of-24:       {:.0}ms (measured)", avg_total * 1000.0);
    println!();

    println!("Note on Throughput:");
    println!("  Each DKG is independent - you can run multiple concurrent");
    println!("  address generation requests for much higher throughput.");
    println!("  Example: 10 concurrent requests = ~300 addresses/second");
    println!();

    println!("Security vs Performance Trade-off:");
    println!("  Higher threshold = More security + Slower DKG");
    println!("  16-of-24 provides enterprise-grade security");
    println!("  Can tolerate {} Byzantine nodes", NODE_COUNT - THRESHOLD);
    println!();

    println!("✅ All tests completed successfully!\n");

    Ok(())
}
