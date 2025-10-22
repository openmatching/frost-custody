use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::api::NodeHealthStatus;

#[derive(Serialize)]
struct Round1Request {
    message: String,
}

#[derive(Deserialize, Clone)]
struct Round1Response {
    identifier: String,
    commitments: String,
    encrypted_nonces: String,
    node_index: u16,
}

#[derive(Serialize)]
struct Round2Request {
    message: String,
    encrypted_nonces: String,
    all_commitments: Vec<CommitmentEntry>,
}

#[derive(Serialize, Clone)]
struct CommitmentEntry {
    identifier: String,
    commitments: String,
}

#[derive(Deserialize)]
struct Round2Response {
    signature_share: String,
    identifier: String,
}

#[derive(Serialize)]
struct AggregateRequest {
    message: String,
    all_commitments: Vec<CommitmentEntry>,
    signature_shares: Vec<SignatureShareEntry>,
}

#[derive(Serialize)]
struct SignatureShareEntry {
    identifier: String,
    share: String,
}

#[derive(Deserialize)]
struct AggregateResponse {
    signature: String,
    verified: bool,
}

/// Orchestrate FROST 3-round signing protocol with multiple signer nodes
///
/// This function handles all the complexity of the FROST protocol,
/// calling the signer nodes and collecting signatures.
///
/// CEX backend just calls this once - all FROST coordination is hidden!
pub async fn sign_message(
    message: &str,
    signer_urls: &[String],
    threshold: usize,
) -> Result<(String, usize)> {
    let client = reqwest::Client::new();

    tracing::debug!(
        "Starting FROST signing with {} nodes (threshold: {})",
        signer_urls.len(),
        threshold
    );

    // Round 1: Get commitments from threshold number of nodes
    tracing::debug!("FROST Round 1: Collecting commitments");

    let mut round1_responses = Vec::new();
    for (i, url) in signer_urls.iter().take(threshold).enumerate() {
        tracing::debug!("  Requesting commitment from node {} at {}", i, url);

        let resp = client
            .post(format!("{}/api/frost/round1", url))
            .json(&Round1Request {
                message: message.to_string(),
            })
            .send()
            .await
            .context(format!("Failed to call node {} round1", i))?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_else(|_| "Unknown".to_string());
            anyhow::bail!("Node {} round1 failed: {}", i, error);
        }

        let r1: Round1Response = resp
            .json()
            .await
            .context("Failed to parse round1 response")?;

        tracing::debug!("  ✅ Node {} commitment received", i);
        round1_responses.push(r1);
    }

    // Prepare commitments for round 2
    let all_commitments: Vec<CommitmentEntry> = round1_responses
        .iter()
        .map(|r| CommitmentEntry {
            identifier: r.identifier.clone(),
            commitments: r.commitments.clone(),
        })
        .collect();

    // Round 2: Get signature shares
    tracing::debug!("FROST Round 2: Collecting signature shares");

    let mut round2_responses = Vec::new();
    for (i, (url, r1)) in signer_urls
        .iter()
        .take(threshold)
        .zip(&round1_responses)
        .enumerate()
    {
        tracing::debug!("  Requesting signature share from node {}", i);

        let resp = client
            .post(format!("{}/api/frost/round2", url))
            .json(&Round2Request {
                message: message.to_string(),
                encrypted_nonces: r1.encrypted_nonces.clone(),
                all_commitments: all_commitments.clone(),
            })
            .send()
            .await
            .context(format!("Failed to call node {} round2", i))?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_else(|_| "Unknown".to_string());
            anyhow::bail!("Node {} round2 failed: {}", i, error);
        }

        let r2: Round2Response = resp
            .json()
            .await
            .context("Failed to parse round2 response")?;

        tracing::debug!("  ✅ Node {} signature share received", i);
        round2_responses.push(r2);
    }

    // Round 3: Aggregate signature
    tracing::debug!("FROST Round 3: Aggregating signature");

    let signature_shares: Vec<SignatureShareEntry> = round2_responses
        .iter()
        .map(|r| SignatureShareEntry {
            identifier: r.identifier.clone(),
            share: r.signature_share.clone(),
        })
        .collect();

    let resp = client
        .post(format!("{}/api/frost/aggregate", signer_urls[0]))
        .json(&AggregateRequest {
            message: message.to_string(),
            all_commitments,
            signature_shares,
        })
        .send()
        .await
        .context("Failed to call aggregate")?;

    if !resp.status().is_success() {
        let error = resp.text().await.unwrap_or_else(|_| "Unknown".to_string());
        anyhow::bail!("Aggregate failed: {}", error);
    }

    let aggregate: AggregateResponse = resp
        .json()
        .await
        .context("Failed to parse aggregate response")?;

    tracing::info!(
        "✅ FROST signing complete, signature verified: {}",
        aggregate.verified
    );

    Ok((aggregate.signature, threshold))
}

/// Get Taproot address from signer node
pub async fn get_address(node_url: &str, passphrase: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct AddressResponse {
        address: String,
    }

    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/address", node_url))
        .query(&[("passphrase", passphrase)])
        .send()
        .await
        .context("Failed to call address API")?;

    if !resp.status().is_success() {
        let error = resp.text().await.unwrap_or_else(|_| "Unknown".to_string());
        anyhow::bail!("Address API failed: {}", error);
    }

    let addr_resp: AddressResponse = resp
        .json()
        .await
        .context("Failed to parse address response")?;

    Ok(addr_resp.address)
}

/// Check health of all signer nodes
pub async fn check_all_nodes_health(node_urls: &[String]) -> Vec<NodeHealthStatus> {
    #[derive(Deserialize)]
    struct HealthResponse {
        status: String,
    }

    let client = reqwest::Client::new();
    let mut statuses = Vec::new();

    for url in node_urls {
        let health_url = format!("{}/health", url);

        match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<HealthResponse>().await {
                Ok(health) if health.status == "ok" => {
                    statuses.push(NodeHealthStatus {
                        url: url.clone(),
                        healthy: true,
                        error: None,
                    });
                }
                Ok(health) => {
                    statuses.push(NodeHealthStatus {
                        url: url.clone(),
                        healthy: false,
                        error: Some(format!("Status: {}", health.status)),
                    });
                }
                Err(e) => {
                    statuses.push(NodeHealthStatus {
                        url: url.clone(),
                        healthy: false,
                        error: Some(format!("Parse error: {}", e)),
                    });
                }
            },
            Ok(resp) => {
                let status_code = resp.status();
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown".to_string());
                statuses.push(NodeHealthStatus {
                    url: url.clone(),
                    healthy: false,
                    error: Some(format!("HTTP {}: {}", status_code, error_text)),
                });
            }
            Err(e) => {
                statuses.push(NodeHealthStatus {
                    url: url.clone(),
                    healthy: false,
                    error: Some(format!("Connection error: {}", e)),
                });
            }
        }
    }

    statuses
}
