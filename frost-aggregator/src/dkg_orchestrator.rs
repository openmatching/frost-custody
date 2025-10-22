use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct DkgRound1Request {
    passphrase: String,
}

#[derive(Deserialize)]
struct DkgRound1Response {
    package: String,
    node_index: u16,
}

#[derive(Serialize, Clone)]
struct DkgRound1Package {
    node_index: u16,
    package: String,
}

#[derive(Serialize)]
struct DkgRound2Request {
    passphrase: String,
    round1_packages: Vec<DkgRound1Package>,
}

#[derive(Deserialize)]
struct DkgRound2Response {
    packages: Vec<DkgPackageEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct DkgPackageEntry {
    sender_index: u16,    // Who sent this package
    recipient_index: u16, // Who it's for
    package: String,
}

#[derive(Serialize)]
struct DkgFinalizeRequest {
    passphrase: String,
    round1_packages: Vec<DkgRound1Package>,
    round2_packages: Vec<DkgPackageEntry>,
}

#[derive(Deserialize)]
struct DkgFinalizeResponse {
    success: bool,
    address: String,
}

/// Orchestrate DKG across all signer nodes to generate new address
pub async fn orchestrate_dkg(signer_urls: &[String], passphrase: &str) -> Result<String> {
    let client = reqwest::Client::new();

    tracing::info!(
        "Starting DKG for passphrase across {} nodes",
        signer_urls.len()
    );

    // DKG Round 1: Collect commitments from all nodes
    tracing::info!("DKG Round 1: Collecting commitments");

    let mut round1_responses = Vec::new();
    for (i, url) in signer_urls.iter().enumerate() {
        tracing::debug!("  Calling node {} at {}", i, url);

        let resp = client
            .post(format!("{}/api/dkg/round1", url))
            .json(&DkgRound1Request {
                passphrase: passphrase.to_string(),
            })
            .send()
            .await
            .context(format!("Failed to call node {} round1", i))?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            anyhow::bail!("Node {} round1 failed: {}", i, error);
        }

        let r1: DkgRound1Response = resp
            .json()
            .await
            .context(format!("Failed to parse round1 response from node {}", i))?;

        tracing::debug!("  ✅ Node {} round1 complete", i);
        round1_responses.push(r1);
    }

    // Prepare round1 packages for broadcast
    let all_round1_packages: Vec<DkgRound1Package> = round1_responses
        .iter()
        .map(|r| DkgRound1Package {
            node_index: r.node_index,
            package: r.package.clone(),
        })
        .collect();

    tracing::info!(
        "✅ DKG Round 1 complete, collected {} packages",
        all_round1_packages.len()
    );

    // DKG Round 2: Each node processes round1 packages and generates round2 packages
    tracing::info!("DKG Round 2: Generating shares");

    let mut all_round2_packages: Vec<Vec<DkgPackageEntry>> = vec![Vec::new(); signer_urls.len()];

    for (i, url) in signer_urls.iter().enumerate() {
        tracing::debug!("  Calling node {} at {}", i, url);

        let resp = client
            .post(format!("{}/api/dkg/round2", url))
            .json(&DkgRound2Request {
                passphrase: passphrase.to_string(),
                round1_packages: all_round1_packages.clone(),
            })
            .send()
            .await
            .context(format!("Failed to call node {} round2", i))?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            anyhow::bail!("Node {} round2 failed: {}", i, error);
        }

        let r2: DkgRound2Response = resp
            .json()
            .await
            .context(format!("Failed to parse round2 response from node {}", i))?;

        // Distribute packages to recipients
        for entry in r2.packages {
            let recipient_idx = entry.recipient_index as usize;
            // Add sender_index for tracking
            let mut entry_with_sender = entry.clone();
            entry_with_sender.sender_index = i as u16; // Current node is the sender
            all_round2_packages[recipient_idx].push(entry_with_sender);
        }

        tracing::debug!("  ✅ Node {} round2 complete", i);
    }

    tracing::info!("✅ DKG Round 2 complete, packages distributed");

    // DKG Finalize: Each node combines packages
    tracing::info!("DKG Finalize: Completing key generation");

    let mut generated_address: Option<String> = None;

    for (i, url) in signer_urls.iter().enumerate() {
        tracing::debug!("  Finalizing on node {} at {}", i, url);

        let resp = client
            .post(format!("{}/api/dkg/finalize", url))
            .json(&DkgFinalizeRequest {
                passphrase: passphrase.to_string(),
                round1_packages: all_round1_packages.clone(),
                round2_packages: all_round2_packages[i].clone(),
            })
            .send()
            .await
            .context(format!("Failed to call node {} finalize", i))?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            anyhow::bail!("Node {} finalize failed: {}", i, error);
        }

        let finalize_resp: DkgFinalizeResponse = resp
            .json()
            .await
            .context(format!("Failed to parse finalize response from node {}", i))?;

        if generated_address.is_none() {
            generated_address = Some(finalize_resp.address.clone());
        }

        tracing::debug!("  ✅ Node {} finalized: {}", i, finalize_resp.address);
    }

    let address = generated_address.context("No address generated")?;

    tracing::info!("✅ DKG Complete! Generated address: {}", address);

    Ok(address)
}
