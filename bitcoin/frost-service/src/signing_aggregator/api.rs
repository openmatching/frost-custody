// Signing Aggregator API - PSBT signing only (HIGH RISK)

use poem_openapi::payload::Json;
use poem_openapi::{ApiResponse, Object, OpenApi};
use std::sync::Arc;

use crate::config::AggregatorConfig;

pub struct Api {
    pub config: Arc<AggregatorConfig>,
}

#[derive(Debug, Object)]
pub struct SignRequest {
    pub passphrase: String,
    pub message: String,
}

#[derive(Debug, Object)]
pub struct SignResponse {
    pub signature: String,
    pub verified: bool,
    pub signers_used: usize,
}

#[derive(Debug, Object)]
pub struct SignPsbtRequest {
    pub psbt: String,
    pub passphrases: Vec<String>,
}

#[derive(Debug, Object)]
pub struct SignPsbtResponse {
    pub psbt: String,
    pub inputs_signed: usize,
}

#[derive(Debug, Object)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Object)]
pub struct NodeHealthStatus {
    pub url: String,
    pub healthy: bool,
    pub error: Option<String>,
}

#[derive(Debug, Object)]
pub struct HealthResponse {
    pub status: String,
    pub signer_nodes_total: usize,
    pub signer_nodes_healthy: usize,
    pub threshold: usize,
    pub nodes: Vec<NodeHealthStatus>,
}

#[derive(ApiResponse)]
pub enum SignResult {
    #[oai(status = 200)]
    Ok(Json<SignResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

#[derive(ApiResponse)]
pub enum SignPsbtResult {
    #[oai(status = 200)]
    Ok(Json<SignPsbtResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

#[OpenApi]
impl Api {
    /// Sign message with FROST threshold (orchestrates 3-round protocol)
    #[oai(path = "/api/sign", method = "post")]
    async fn sign(&self, req: Json<SignRequest>) -> SignResult {
        let passphrase = req.0.passphrase;
        let message = req.0.message;

        tracing::info!(
            "⚠️  HIGH RISK: Signing request for message: {}...",
            &message[..16.min(message.len())]
        );

        // Sign with passphrase-specific FROST shares
        match crate::common::frost_client::sign_message(
            &passphrase,
            &message,
            &self.config.signer_nodes,
            self.config.threshold,
        )
        .await
        {
            Ok((signature, signers_used)) => {
                tracing::info!("✅ Successfully signed with {} nodes", signers_used);

                SignResult::Ok(Json(SignResponse {
                    signature,
                    verified: true,
                    signers_used,
                }))
            }
            Err(e) => {
                tracing::error!("❌ Signing failed: {}", e);
                SignResult::InternalError(Json(ErrorResponse {
                    error: format!("Failed to sign: {}", e),
                }))
            }
        }
    }

    /// Sign PSBT with FROST threshold signatures (Taproot key-path spend)
    #[oai(path = "/api/sign/psbt", method = "post")]
    async fn sign_psbt(&self, req: Json<SignPsbtRequest>) -> SignPsbtResult {
        let psbt_b64 = req.0.psbt;
        let passphrases = req.0.passphrases;

        tracing::warn!(
            "⚠️  HIGH RISK: PSBT signing request with {} passphrases",
            passphrases.len()
        );

        // Orchestrate PSBT signing
        match crate::common::frost_client::sign_psbt(
            &psbt_b64,
            &passphrases,
            &self.config.signer_nodes,
            self.config.threshold,
        )
        .await
        {
            Ok((signed_psbt_b64, inputs_signed)) => {
                tracing::info!("✅ Successfully signed {} inputs", inputs_signed);

                SignPsbtResult::Ok(Json(SignPsbtResponse {
                    psbt: signed_psbt_b64,
                    inputs_signed,
                }))
            }
            Err(e) => {
                tracing::error!("❌ PSBT signing failed: {}", e);
                SignPsbtResult::InternalError(Json(ErrorResponse {
                    error: format!("Failed to sign PSBT: {}", e),
                }))
            }
        }
    }

    /// Health check (checks all signer nodes)
    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> Json<HealthResponse> {
        let node_statuses_raw = crate::common::frost_client::check_all_nodes_health(&self.config.signer_nodes).await;
        
        // Convert to local type
        let node_statuses: Vec<NodeHealthStatus> = node_statuses_raw.into_iter().map(|n| NodeHealthStatus {
            url: n.url,
            healthy: n.healthy,
            error: n.error,
        }).collect();

        let healthy_count = node_statuses.iter().filter(|n| n.healthy).count();

        let overall_status = if healthy_count >= self.config.threshold {
            "ok".to_string()
        } else {
            format!(
                "degraded: only {} of {} nodes healthy (need {})",
                healthy_count,
                self.config.signer_nodes.len(),
                self.config.threshold
            )
        };

        Json(HealthResponse {
            status: overall_status,
            signer_nodes_total: self.config.signer_nodes.len(),
            signer_nodes_healthy: healthy_count,
            threshold: self.config.threshold,
            nodes: node_statuses,
        })
    }
}

