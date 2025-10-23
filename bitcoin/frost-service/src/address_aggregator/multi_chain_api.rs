//! Multi-chain address aggregator API
//!
//! This is the "smart" layer that:
//! - Orchestrates DKG across signer nodes
//! - Fetches raw public keys from signers
//! - Applies chain-specific address derivation
//! - Handles all chain-specific business logic
//!
//! Signer nodes are "dumb" and only know about curves.

use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{ApiResponse, Object, OpenApi};
use std::sync::Arc;

use super::chain_derivation::{
    derive_bitcoin_address, derive_ethereum_address, derive_solana_address, Chain,
};
use crate::config::AggregatorConfig;

pub struct MultiChainAggregatorApi {
    pub config: Arc<AggregatorConfig>,
}

#[derive(Debug, Object)]
pub struct AddressRequest {
    /// Chain identifier (bitcoin, ethereum, solana)
    pub chain: String,
    /// Unique passphrase for address generation
    pub passphrase: String,
}

#[derive(Debug, Object)]
pub struct AddressResponse {
    pub chain: String,
    pub passphrase: String,
    pub address: String,
    pub curve: String,
}

#[derive(Debug, Object)]
pub struct HealthResponse {
    pub status: String,
    pub supported_chains: Vec<String>,
    pub signer_nodes: usize,
}

#[derive(Debug, ApiResponse)]
enum AddressResult {
    #[oai(status = 200)]
    Ok(Json<AddressResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

#[derive(Debug, Object)]
struct ErrorResponse {
    error: String,
}

#[OpenApi]
impl MultiChainAggregatorApi {
    /// Generate address for any supported chain
    ///
    /// This endpoint:
    /// 1. Determines which curve the chain uses
    /// 2. Ensures DKG has been run for that curve + passphrase
    /// 3. Fetches raw public key from a signer node
    /// 4. Applies chain-specific address derivation locally
    #[oai(path = "/api/address/generate", method = "post")]
    async fn generate_address(&self, Json(req): Json<AddressRequest>) -> AddressResult {
        // Parse chain
        let chain = match Chain::from_str(&req.chain) {
            Ok(c) => c,
            Err(e) => {
                return AddressResult::BadRequest(Json(ErrorResponse {
                    error: format!("Invalid chain: {}", e),
                }))
            }
        };

        // Determine curve and DKG endpoint
        let (curve_name, curve_endpoint) = match chain {
            Chain::Bitcoin | Chain::Ethereum => ("secp256k1", "secp256k1"),
            Chain::Solana => ("ed25519", "ed25519"),
        };

        // Step 1: Check if DKG has been run (try to fetch pubkey from first node)
        let client = reqwest::Client::new();
        let first_node_url = &self.config.signer_urls()[0];

        let pubkey_response = client
            .get(format!(
                "{}/api/curve/{}/pubkey",
                first_node_url, curve_endpoint
            ))
            .query(&[("passphrase", &req.passphrase)])
            .send()
            .await;

        let pubkey_hex = match pubkey_response {
            Ok(resp) if resp.status().is_success() => {
                // DKG already done, fetch the pubkey
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => json["public_key"].as_str().unwrap_or("").to_string(),
                    Err(e) => {
                        return AddressResult::InternalError(Json(ErrorResponse {
                            error: format!("Failed to parse pubkey response: {}", e),
                        }))
                    }
                }
            }
            _ => {
                // DKG not done yet - orchestrate it automatically
                tracing::info!(
                    "DKG not found for curve {} and passphrase, running DKG now...",
                    curve_name
                );

                match super::dkg_orchestrator::orchestrate_dkg(
                    &self.config.signer_urls(),
                    &req.passphrase,
                )
                .await
                {
                    Ok(pubkey) => {
                        tracing::info!(
                            "✅ DKG complete for {}, pubkey: {}...",
                            curve_name,
                            &pubkey[..16]
                        );
                        pubkey
                    }
                    Err(e) => {
                        return AddressResult::InternalError(Json(ErrorResponse {
                            error: format!("DKG orchestration failed: {}", e),
                        }));
                    }
                }
            }
        };

        if pubkey_hex.is_empty() {
            return AddressResult::InternalError(Json(ErrorResponse {
                error: "Empty public key returned".to_string(),
            }));
        }

        // Step 2: Derive chain-specific address from raw pubkey
        let address = match chain {
            Chain::Bitcoin => match derive_bitcoin_address(&pubkey_hex, self.config.network()) {
                Ok(addr) => addr,
                Err(e) => {
                    return AddressResult::InternalError(Json(ErrorResponse {
                        error: format!("Bitcoin address derivation failed: {}", e),
                    }))
                }
            },
            Chain::Ethereum => match derive_ethereum_address(&pubkey_hex) {
                Ok(addr) => addr,
                Err(e) => {
                    return AddressResult::InternalError(Json(ErrorResponse {
                        error: format!("Ethereum address derivation failed: {}", e),
                    }))
                }
            },
            Chain::Solana => match derive_solana_address(&pubkey_hex) {
                Ok(addr) => addr,
                Err(e) => {
                    return AddressResult::InternalError(Json(ErrorResponse {
                        error: format!("Solana address derivation failed: {}", e),
                    }))
                }
            },
        };

        AddressResult::Ok(Json(AddressResponse {
            chain: chain.as_str().to_string(),
            passphrase: req.passphrase,
            address,
            curve: curve_name.to_string(),
        }))
    }

    /// Get address for existing passphrase (no DKG)
    ///
    /// Fast path: just fetch pubkey and derive address
    #[oai(path = "/api/address", method = "get")]
    async fn get_address(
        &self,
        Query(chain): Query<String>,
        Query(passphrase): Query<String>,
    ) -> AddressResult {
        self.generate_address(Json(AddressRequest { chain, passphrase }))
            .await
    }

    /// Health check
    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> Json<HealthResponse> {
        Json(HealthResponse {
            status: "ok".to_string(),
            supported_chains: vec![
                "bitcoin".to_string(),
                "ethereum".to_string(),
                "solana".to_string(),
            ],
            signer_nodes: self.config.signer_urls().len(),
        })
    }
}

// ============================================================================
// Usage Examples
// ============================================================================

// Generate Bitcoin address:
// POST /api/address/generate
// {
//   "chain": "bitcoin",
//   "passphrase": "550e8400-e29b-41d4-a716-446655440000"
// }
//
// Generate Ethereum address (same FROST key as Bitcoin!):
// POST /api/address/generate
// {
//   "chain": "ethereum",
//   "passphrase": "550e8400-e29b-41d4-a716-446655440000"
// }
//
// Generate Solana address (separate FROST key):
// POST /api/address/generate
// {
//   "chain": "solana",
//   "passphrase": "550e8400-e29b-41d4-a716-446655440000"
// }
//
// Or use query params:
// GET /api/address?chain=bitcoin&passphrase=550e8400-e29b-41d4-a716-446655440000
