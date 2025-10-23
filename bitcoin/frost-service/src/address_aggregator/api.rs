// Address Aggregator API - DKG orchestration only (LOW RISK)

use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{ApiResponse, Object, OpenApi};
use std::sync::Arc;

use crate::config::AggregatorConfig;

pub struct Api {
    pub config: Arc<AggregatorConfig>,
}

#[derive(Debug, Object)]
pub struct AddressResponse {
    pub passphrase: String,
    pub address: String,
    pub script_type: String,
}

#[derive(Debug, Object)]
pub struct GenerateAddressRequest {
    pub passphrase: String,
}

#[derive(Debug, Object)]
pub struct GenerateAddressResponse {
    pub address: String,
    pub passphrase: String,
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

#[OpenApi]
impl Api {
    /// Generate new Taproot address via DKG
    #[oai(path = "/api/address/generate", method = "post")]
    async fn generate_address(&self, req: Json<GenerateAddressRequest>) -> GenerateAddressResult {
        let passphrase = req.0.passphrase;

        tracing::info!("Generating address via DKG for passphrase");

        // Orchestrate DKG across all signer nodes
        match crate::address_aggregator::dkg_orchestrator::orchestrate_dkg(&self.config.signer_nodes, &passphrase).await
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
        match crate::common::frost_client::get_address(&self.config.signer_nodes[0], &passphrase_str).await {
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

    /// Health check (checks all signer nodes)
    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> Json<HealthResponse> {
        // Check health of all signer nodes
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

