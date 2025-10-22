use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{ApiResponse, Object, OpenApi};
use std::sync::Arc;

use crate::config::AggregatorConfig;
use crate::frost_client;

pub struct Api {
    pub config: Arc<AggregatorConfig>,
}

#[derive(Debug, Object)]
pub struct SignRequest {
    /// Passphrase for loading the correct FROST shares
    pub passphrase: String,
    /// Message to sign (hex-encoded, e.g., Bitcoin sighash)
    pub message: String,
}

#[derive(Debug, Object)]
pub struct SignResponse {
    /// Final Schnorr signature (hex-encoded)
    pub signature: String,
    /// Signature verified
    pub verified: bool,
    /// Number of nodes participated
    pub signers_used: usize,
}

#[derive(Debug, Object)]
pub struct AddressResponse {
    /// Passphrase
    pub passphrase: String,
    /// Taproot address
    pub address: String,
    /// Script type
    pub script_type: String,
}

#[derive(Debug, Object)]
pub struct NodeHealthStatus {
    /// Node URL
    pub url: String,
    /// Is healthy
    pub healthy: bool,
    /// Error message (if unhealthy)
    pub error: Option<String>,
}

#[derive(Debug, Object)]
pub struct HealthResponse {
    /// Overall status
    pub status: String,
    /// Number of signer nodes configured
    pub signer_nodes_total: usize,
    /// Number of healthy nodes
    pub signer_nodes_healthy: usize,
    /// Threshold
    pub threshold: usize,
    /// Individual node status
    pub nodes: Vec<NodeHealthStatus>,
}

#[derive(Debug, Object)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
}

#[derive(Debug, Object)]
pub struct GenerateAddressRequest {
    /// Passphrase for address derivation
    pub passphrase: String,
}

#[derive(Debug, Object)]
pub struct GenerateAddressResponse {
    /// Generated Taproot address
    pub address: String,
    /// Passphrase
    pub passphrase: String,
}

#[derive(Debug, Object)]
pub struct SignPsbtRequest {
    /// Base64-encoded PSBT
    pub psbt: String,
    /// Passphrases for each input (one per input, in order)
    pub passphrases: Vec<String>,
}

#[derive(Debug, Object)]
pub struct SignPsbtResponse {
    /// Signed PSBT (base64-encoded)
    pub psbt: String,
    /// Number of inputs signed
    pub inputs_signed: usize,
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
pub enum GenerateAddressResult {
    #[oai(status = 200)]
    Ok(Json<GenerateAddressResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

#[derive(ApiResponse)]
pub enum AddressResult {
    #[oai(status = 200)]
    Ok(Json<AddressResponse>),
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
            "Received signing request for message: {}...",
            &message[..16.min(message.len())]
        );

        // Sign with passphrase-specific FROST shares
        match frost_client::sign_message(
            &passphrase,
            &message,
            &self.config.signer_nodes,
            self.config.threshold,
        )
        .await
        {
            Ok((signature, signers_used)) => {
                tracing::info!("Successfully signed with {} nodes", signers_used);

                SignResult::Ok(Json(SignResponse {
                    signature,
                    verified: true,
                    signers_used,
                }))
            }
            Err(e) => {
                tracing::error!("Signing failed: {}", e);
                SignResult::InternalError(Json(ErrorResponse {
                    error: format!("Failed to sign: {}", e),
                }))
            }
        }
    }

    /// Generate new Taproot address via DKG
    #[oai(path = "/api/address/generate", method = "post")]
    async fn generate_address(&self, req: Json<GenerateAddressRequest>) -> GenerateAddressResult {
        let passphrase = req.0.passphrase;

        tracing::info!("Generating address via DKG for passphrase");

        // Orchestrate DKG across all signer nodes
        match crate::dkg_orchestrator::orchestrate_dkg(&self.config.signer_nodes, &passphrase).await
        {
            Ok(address) => {
                tracing::info!("✅ DKG complete, generated address: {}", address);

                GenerateAddressResult::Ok(Json(GenerateAddressResponse {
                    address,
                    passphrase,
                }))
            }
            Err(e) => {
                tracing::error!("DKG orchestration failed: {}", e);
                GenerateAddressResult::InternalError(Json(ErrorResponse {
                    error: format!("Failed to generate address: {}", e),
                }))
            }
        }
    }

    /// Get Taproot address for passphrase (proxies to signer to check cache)
    #[oai(path = "/api/address", method = "get")]
    async fn get_address(&self, passphrase: Query<String>) -> AddressResult {
        let passphrase_str = passphrase.0;

        // Try first signer node (it will use cache if available)
        match frost_client::get_address(&self.config.signer_nodes[0], &passphrase_str).await {
            Ok(address) => AddressResult::Ok(Json(AddressResponse {
                passphrase: passphrase_str,
                address,
                script_type: "p2tr".to_string(),
            })),
            Err(e) => {
                tracing::error!("Failed to get address: {}", e);
                AddressResult::InternalError(Json(ErrorResponse {
                    error: format!("Address not found. Use POST /api/address/generate to create new address via DKG: {}", e),
                }))
            }
        }
    }

    /// Sign PSBT with FROST threshold signatures (Taproot key-path spend)
    #[oai(path = "/api/sign/psbt", method = "post")]
    async fn sign_psbt(&self, req: Json<SignPsbtRequest>) -> SignPsbtResult {
        let psbt_b64 = req.0.psbt;
        let passphrases = req.0.passphrases;

        tracing::info!(
            "Received PSBT signing request with {} passphrases",
            passphrases.len()
        );

        // Orchestrate PSBT signing
        match frost_client::sign_psbt(
            &psbt_b64,
            &passphrases,
            &self.config.signer_nodes,
            self.config.threshold,
        )
        .await
        {
            Ok((signed_psbt_b64, inputs_signed)) => {
                tracing::info!("Successfully signed {} inputs", inputs_signed);

                SignPsbtResult::Ok(Json(SignPsbtResponse {
                    psbt: signed_psbt_b64,
                    inputs_signed,
                }))
            }
            Err(e) => {
                tracing::error!("PSBT signing failed: {}", e);
                SignPsbtResult::InternalError(Json(ErrorResponse {
                    error: format!("Failed to sign PSBT: {}", e),
                }))
            }
        }
    }

    /// Health check (checks all signer nodes)
    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> Json<HealthResponse> {
        // Check health of all signer nodes
        let node_statuses = frost_client::check_all_nodes_health(&self.config.signer_nodes).await;

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
