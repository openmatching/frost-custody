use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{ApiResponse, Object, OpenApi};
use std::sync::Arc;

use crate::config::SignerNode;
use crate::signer;

pub struct Api {
    pub config: Arc<SignerNode>,
}

#[derive(Debug, Object)]
pub struct SignRequest {
    /// Base64-encoded PSBT
    pub psbt: String,
    /// Passphrases for key derivation (one per input).
    /// Each element corresponds to a PSBT input by index.
    /// Use UUIDs, random strings, or hashed user identifiers (NOT sequential IDs!).
    pub passphrases: Vec<String>,
}

#[derive(Debug, Object)]
pub struct SignResponse {
    /// Base64-encoded signed PSBT
    pub psbt: String,
    /// Number of inputs successfully signed
    pub signed_count: usize,
    /// This node's index
    pub node_index: u8,
}

#[derive(Debug, Object)]
pub struct PubkeyResponse {
    /// Passphrase (returned for confirmation)
    pub passphrase: String,
    /// Compressed public key (hex)
    pub pubkey: String,
    /// This node's index
    pub node_index: u8,
}

#[derive(Debug, Object)]
pub struct AddressResponse {
    /// Passphrase (returned for confirmation)
    pub passphrase: String,
    /// 2-of-3 multisig address
    pub address: String,
    /// Script type
    pub script_type: String,
}

#[derive(Debug, Object)]
pub struct HealthResponse {
    /// Status
    pub status: String,
    /// This node's index
    pub node_index: u8,
    /// Account xpub
    pub xpub: String,
}

#[derive(Debug, Object)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
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
pub enum PubkeyResult {
    #[oai(status = 200)]
    Ok(Json<PubkeyResponse>),
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
    /// Sign a PSBT
    #[oai(path = "/api/sign", method = "post")]
    async fn sign(&self, req: Json<SignRequest>) -> SignResult {
        let req = req.0;

        // Validate passphrases
        if req.passphrases.is_empty() {
            return SignResult::BadRequest(Json(ErrorResponse {
                error: "passphrases cannot be empty - must provide passphrase for each input"
                    .to_string(),
            }));
        }

        // Sign PSBT
        match signer::sign_psbt(&self.config, &req.psbt, &req.passphrases) {
            Ok((signed_psbt, signed_count)) => {
                tracing::info!(
                    "Signed {} inputs for node {}",
                    signed_count,
                    self.config.node_index
                );

                SignResult::Ok(Json(SignResponse {
                    psbt: signed_psbt,
                    signed_count,
                    node_index: self.config.node_index,
                }))
            }
            Err(e) => {
                tracing::error!("Sign error: {}", e);
                SignResult::InternalError(Json(ErrorResponse {
                    error: format!("Failed to sign PSBT: {}", e),
                }))
            }
        }
    }

    /// Get public key for a passphrase
    #[oai(path = "/api/pubkey", method = "get")]
    async fn get_pubkey(&self, passphrase: Query<String>) -> PubkeyResult {
        let passphrase_str = passphrase.0;

        match self.config.derive_pubkey(&passphrase_str) {
            Ok(pubkey) => PubkeyResult::Ok(Json(PubkeyResponse {
                passphrase: passphrase_str,
                pubkey: pubkey.to_string(),
                node_index: self.config.node_index,
            })),
            Err(e) => {
                tracing::error!("Failed to derive pubkey: {}", e);
                PubkeyResult::InternalError(Json(ErrorResponse {
                    error: format!("Failed to derive pubkey: {}", e),
                }))
            }
        }
    }

    /// Get multisig address for a passphrase
    #[oai(path = "/api/address", method = "get")]
    async fn get_address(&self, passphrase: Query<String>) -> AddressResult {
        let passphrase_str = passphrase.0;

        match signer::derive_multisig_address(&self.config, &passphrase_str) {
            Ok(address) => AddressResult::Ok(Json(AddressResponse {
                passphrase: passphrase_str,
                address,
                script_type: "wsh_sortedmulti(2,3)".to_string(),
            })),
            Err(e) => {
                tracing::error!("Failed to derive address: {}", e);
                AddressResult::InternalError(Json(ErrorResponse {
                    error: format!("Failed to derive address: {}", e),
                }))
            }
        }
    }

    /// Health check
    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> Json<HealthResponse> {
        Json(HealthResponse {
            status: "ok".to_string(),
            node_index: self.config.node_index,
            xpub: self.config.account_xpub.to_string(),
        })
    }
}
