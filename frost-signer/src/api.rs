use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{ApiResponse, Object, OpenApi};
use std::sync::Arc;

use crate::config::FrostNode;

pub struct Api {
    pub config: Arc<FrostNode>,
}

#[derive(Debug, Object)]
pub struct AddressResponse {
    /// Passphrase
    pub passphrase: String,
    /// Taproot address (P2TR)
    pub address: String,
    /// Script type
    pub script_type: String,
}

#[derive(Debug, Object)]
pub struct CommitmentsRequest {
    /// Message to sign (hex)
    pub message: String,
}

#[derive(Debug, Object)]
pub struct CommitmentsResponse {
    /// This node's identifier
    pub identifier: String,
    /// Signing commitments (hex)
    pub commitments: String,
    /// Encrypted nonces (return this in round2)
    pub encrypted_nonces: String,
    /// Node index
    pub node_index: u16,
}

#[derive(Debug, Object)]
pub struct SignRequest {
    /// Message to sign (hex - same as round 1)
    pub message: String,
    /// Encrypted nonces (from round 1 response)
    pub encrypted_nonces: String,
    /// All signing commitments from round 1
    pub all_commitments: Vec<CommitmentEntry>,
}

#[derive(Debug, Object, Clone)]
pub struct CommitmentEntry {
    pub identifier: String,
    pub commitments: String,
}

#[derive(Debug, Object)]
pub struct SignResponse {
    /// This node's signature share (hex)
    pub signature_share: String,
    /// Node identifier
    pub identifier: String,
}

#[derive(Debug, Object)]
pub struct AggregateRequest {
    /// Message that was signed (hex)
    pub message: String,
    /// All commitments from round 1
    pub all_commitments: Vec<CommitmentEntry>,
    /// All signature shares from round 2
    pub signature_shares: Vec<SignatureShareEntry>,
}

#[derive(Debug, Object, Clone)]
pub struct SignatureShareEntry {
    pub identifier: String,
    pub share: String,
}

#[derive(Debug, Object)]
pub struct AggregateResponse {
    /// Final Schnorr signature (hex)
    pub signature: String,
    /// Verified
    pub verified: bool,
}

#[derive(Debug, Object)]
pub struct HealthResponse {
    /// Status
    pub status: String,
    /// This node's index
    pub node_index: u16,
    /// Group public key (hex)
    pub group_pubkey: String,
    /// Signing mode
    pub mode: String,
}

#[derive(Debug, Object)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
}

#[derive(ApiResponse)]
pub enum AddressResult {
    #[oai(status = 200)]
    Ok(Json<AddressResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

#[derive(ApiResponse)]
pub enum CommitmentsResult {
    #[oai(status = 200)]
    Ok(Json<CommitmentsResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
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
pub enum AggregateResult {
    #[oai(status = 200)]
    Ok(Json<AggregateResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

#[OpenApi]
impl Api {
    /// Get Taproot address for a passphrase
    #[oai(path = "/api/address", method = "get")]
    async fn get_address(&self, passphrase: Query<String>) -> AddressResult {
        let passphrase_str = passphrase.0;

        match self.config.get_taproot_address(&passphrase_str) {
            Ok(address) => AddressResult::Ok(Json(AddressResponse {
                passphrase: passphrase_str,
                address,
                script_type: "p2tr".to_string(),
            })),
            Err(e) => {
                tracing::error!("Failed to derive address: {}", e);
                AddressResult::InternalError(Json(ErrorResponse {
                    error: format!("Failed to derive address: {}", e),
                }))
            }
        }
    }

    /// Round 1: Generate signing commitments
    #[oai(path = "/api/frost/round1", method = "post")]
    async fn round1(&self, req: Json<CommitmentsRequest>) -> CommitmentsResult {
        let message = match hex::decode(&req.0.message) {
            Ok(msg) => msg,
            Err(e) => {
                return CommitmentsResult::InternalError(Json(ErrorResponse {
                    error: format!("Invalid message hex: {}", e),
                }))
            }
        };

        match crate::signer::generate_commitments(&self.config) {
            Ok((nonces, commitments)) => {
                // Serialize commitments
                let commitments_json = serde_json::to_vec(&commitments).unwrap();

                // Serialize and encrypt nonces (bound to message)
                let nonces_json = serde_json::to_vec(&nonces).unwrap();
                let encrypted_nonces = match crate::crypto::encrypt_nonces(
                    &nonces_json,
                    &message,
                    &self.get_key_package_hex(),
                ) {
                    Ok(enc) => enc,
                    Err(e) => {
                        return CommitmentsResult::InternalError(Json(ErrorResponse {
                            error: format!("Failed to encrypt nonces: {}", e),
                        }))
                    }
                };

                CommitmentsResult::Ok(Json(CommitmentsResponse {
                    identifier: format!("{:?}", self.config.identifier),
                    commitments: hex::encode(commitments_json),
                    encrypted_nonces,
                    node_index: self.config.node_index,
                }))
            }
            Err(e) => CommitmentsResult::InternalError(Json(ErrorResponse {
                error: format!("Failed to generate commitments: {}", e),
            })),
        }
    }

    /// Round 2: Sign with commitments and encrypted nonces
    #[oai(path = "/api/frost/round2", method = "post")]
    async fn round2(&self, req: Json<SignRequest>) -> SignResult {
        let req = req.0;

        // Decode message
        let message = match hex::decode(&req.message) {
            Ok(msg) => msg,
            Err(e) => {
                return SignResult::BadRequest(Json(ErrorResponse {
                    error: format!("Invalid message hex: {}", e),
                }))
            }
        };

        // Decrypt nonces (verifies message binding)
        let nonces_json = match crate::crypto::decrypt_nonces(
            &req.encrypted_nonces,
            &message,
            &self.get_key_package_hex(),
        ) {
            Ok(json) => json,
            Err(e) => {
                return SignResult::BadRequest(Json(ErrorResponse {
                    error: format!("Failed to decrypt nonces: {}", e),
                }))
            }
        };

        let nonces: frost_secp256k1::round1::SigningNonces =
            match serde_json::from_slice(&nonces_json) {
                Ok(n) => n,
                Err(e) => {
                    return SignResult::InternalError(Json(ErrorResponse {
                        error: format!("Failed to deserialize nonces: {}", e),
                    }))
                }
            };

        // Parse commitments
        let mut commitments_map = std::collections::BTreeMap::new();
        for entry in req.all_commitments {
            let id_str = entry.identifier.trim_matches('"');
            let identifier = match frost_secp256k1::Identifier::deserialize(
                &hex::decode(id_str).unwrap_or_default(),
            ) {
                Ok(id) => id,
                Err(_) => continue,
            };

            let comm_json = match hex::decode(&entry.commitments) {
                Ok(j) => j,
                Err(_) => continue,
            };

            let commitments: frost_secp256k1::round1::SigningCommitments =
                match serde_json::from_slice(&comm_json) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

            commitments_map.insert(identifier, commitments);
        }

        // Sign
        match crate::signer::sign_with_commitments(&self.config, &message, &nonces, commitments_map)
        {
            Ok(signature_share) => {
                let share_json = serde_json::to_vec(&signature_share).unwrap();

                SignResult::Ok(Json(SignResponse {
                    signature_share: hex::encode(share_json),
                    identifier: format!("{:?}", self.config.identifier),
                }))
            }
            Err(e) => SignResult::InternalError(Json(ErrorResponse {
                error: format!("Failed to sign: {}", e),
            })),
        }
    }

    /// Aggregate signature shares into final signature
    #[oai(path = "/api/frost/aggregate", method = "post")]
    async fn aggregate(&self, req: Json<AggregateRequest>) -> AggregateResult {
        let req = req.0;

        // Decode message
        let message = match hex::decode(&req.message) {
            Ok(msg) => msg,
            Err(e) => {
                return AggregateResult::BadRequest(Json(ErrorResponse {
                    error: format!("Invalid message hex: {}", e),
                }))
            }
        };

        // Parse commitments
        let mut commitments_map = std::collections::BTreeMap::new();
        for entry in req.all_commitments {
            let id_str = entry.identifier.trim_matches('"');
            let identifier = match frost_secp256k1::Identifier::deserialize(
                &hex::decode(id_str).unwrap_or_default(),
            ) {
                Ok(id) => id,
                Err(_) => continue,
            };

            let comm_json = match hex::decode(&entry.commitments) {
                Ok(j) => j,
                Err(_) => continue,
            };

            let commitments: frost_secp256k1::round1::SigningCommitments =
                match serde_json::from_slice(&comm_json) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

            commitments_map.insert(identifier, commitments);
        }

        // Parse signature shares
        let mut shares_map = std::collections::BTreeMap::new();
        for entry in req.signature_shares {
            let id_str = entry.identifier.trim_matches('"');
            let identifier = match frost_secp256k1::Identifier::deserialize(
                &hex::decode(id_str).unwrap_or_default(),
            ) {
                Ok(id) => id,
                Err(_) => continue,
            };

            let share_json = match hex::decode(&entry.share) {
                Ok(j) => j,
                Err(_) => continue,
            };

            let share: frost_secp256k1::round2::SignatureShare =
                match serde_json::from_slice(&share_json) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

            shares_map.insert(identifier, share);
        }

        // Aggregate
        match crate::signer::aggregate_signature(
            &self.config,
            &message,
            commitments_map,
            shares_map,
        ) {
            Ok(signature) => {
                // Verify signature
                let verified = crate::signer::verify_signature(&self.config, &message, &signature)
                    .unwrap_or(false);

                let sig_bytes = signature
                    .serialize()
                    .map_err(|e| format!("Failed to serialize signature: {:?}", e))
                    .unwrap_or_default();

                AggregateResult::Ok(Json(AggregateResponse {
                    signature: hex::encode(&sig_bytes),
                    verified,
                }))
            }
            Err(e) => AggregateResult::InternalError(Json(ErrorResponse {
                error: format!("Failed to aggregate: {}", e),
            })),
        }
    }

    fn get_key_package_hex(&self) -> String {
        // Serialize key package for encryption key
        serde_json::to_vec(&self.config.key_package)
            .ok()
            .map(hex::encode)
            .unwrap_or_default()
    }

    /// Health check
    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> Json<HealthResponse> {
        let group_pubkey_bytes = self
            .config
            .pubkey_package
            .verifying_key()
            .serialize()
            .unwrap_or_default();

        Json(HealthResponse {
            status: "ok".to_string(),
            node_index: self.config.node_index,
            group_pubkey: hex::encode(&group_pubkey_bytes),
            mode: "FROST threshold signature".to_string(),
        })
    }
}
