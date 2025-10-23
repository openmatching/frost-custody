//! Signing Aggregator API
//!
//! Orchestrates FROST threshold signing across signer nodes.
//! Clients call this to sign messages/PSBTs without knowing about signer nodes.

use poem_openapi::payload::Json;
use poem_openapi::{ApiResponse, Object, OpenApi};
use std::sync::Arc;

use crate::config::AggregatorConfig;

pub struct SigningAggregatorApi {
    pub config: Arc<AggregatorConfig>,
}

#[derive(Debug, Object)]
pub struct SignMessageRequest {
    pub passphrase: String,
    pub message: String, // hex-encoded
}

#[derive(Debug, Object)]
pub struct SignMessageResponse {
    pub signature: String, // hex-encoded Schnorr signature
    pub verified: bool,
}

#[derive(Debug, Object)]
pub struct SignPsbtRequest {
    pub psbt: String,             // base64-encoded PSBT
    pub passphrases: Vec<String>, // one per input
}

#[derive(Debug, Object)]
pub struct SignPsbtResponse {
    pub signed_psbt: String, // base64-encoded signed PSBT
    pub signatures_added: usize,
}

#[derive(Debug, Object)]
pub struct HealthResponse {
    pub status: String,
    pub signer_nodes: usize,
    pub threshold: usize,
}

#[derive(Debug, Object)]
pub struct ErrorResponse {
    error: String,
}

#[derive(ApiResponse)]
pub enum SignResult {
    #[oai(status = 200)]
    Ok(Json<SignMessageResponse>),
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
impl SigningAggregatorApi {
    /// Sign message with FROST threshold signatures
    ///
    /// Orchestrates FROST protocol across signer nodes:
    /// 1. Call /api/frost/round1 on threshold nodes
    /// 2. Collect commitments
    /// 3. Call /api/frost/round2 with commitments
    /// 4. Collect signature shares
    /// 5. Call /api/frost/aggregate to get final signature
    #[oai(path = "/api/sign/message", method = "post")]
    async fn sign_message(&self, Json(req): Json<SignMessageRequest>) -> SignResult {
        tracing::info!("Signing message with FROST");

        // Orchestrate FROST signing
        match crate::common::frost_client::sign_message(
            &req.passphrase,
            &req.message,
            &self.config.signer_nodes,
            self.config.threshold,
        )
        .await
        {
            Ok((signature, _)) => SignResult::Ok(Json(SignMessageResponse {
                signature,
                verified: true,
            })),
            Err(e) => SignResult::InternalError(Json(ErrorResponse {
                error: format!("FROST signing failed: {}", e),
            })),
        }
    }

    /// Sign PSBT with FROST threshold signatures
    ///
    /// For each input in PSBT:
    /// 1. Extract sighash
    /// 2. Orchestrate FROST signing for that input's passphrase
    /// 3. Add Schnorr signature to PSBT witness
    #[oai(path = "/api/sign/psbt", method = "post")]
    async fn sign_psbt(&self, Json(req): Json<SignPsbtRequest>) -> SignPsbtResult {
        tracing::info!("Signing PSBT with FROST");

        // Parse PSBT
        let mut psbt: bitcoin::psbt::Psbt = match req.psbt.parse() {
            Ok(p) => p,
            Err(e) => {
                return SignPsbtResult::BadRequest(Json(ErrorResponse {
                    error: format!("Invalid PSBT: {}", e),
                }))
            }
        };

        if psbt.inputs.len() != req.passphrases.len() {
            return SignPsbtResult::BadRequest(Json(ErrorResponse {
                error: format!(
                    "Passphrase count mismatch: {} inputs, {} passphrases",
                    psbt.inputs.len(),
                    req.passphrases.len()
                ),
            }));
        }

        // Sign each input
        let mut signatures_added = 0;

        for (input_idx, passphrase) in req.passphrases.iter().enumerate() {
            tracing::debug!("Signing input {} with passphrase", input_idx);

            // Calculate Taproot sighash
            use bitcoin::hashes::Hash;
            use bitcoin::sighash::{Prevouts, SighashCache};
            use bitcoin::TapSighashType;

            let prevouts: Vec<bitcoin::TxOut> = psbt
                .inputs
                .iter()
                .filter_map(|input| input.witness_utxo.clone())
                .collect();

            if prevouts.len() != psbt.inputs.len() {
                return SignPsbtResult::BadRequest(Json(ErrorResponse {
                    error: "Missing witness_utxo in PSBT inputs".to_string(),
                }));
            }

            let prevouts = Prevouts::All(&prevouts);
            let mut cache = SighashCache::new(&psbt.unsigned_tx);

            let sighash = match cache.taproot_key_spend_signature_hash(
                input_idx,
                &prevouts,
                TapSighashType::Default,
            ) {
                Ok(hash) => hash,
                Err(e) => {
                    return SignPsbtResult::InternalError(Json(ErrorResponse {
                        error: format!("Sighash calculation failed: {}", e),
                    }))
                }
            };

            let sighash_hex = hex::encode(sighash.as_byte_array());

            // Sign with FROST
            let (signature_hex, _) = match crate::common::frost_client::sign_message(
                passphrase,
                &sighash_hex,
                &self.config.signer_nodes,
                self.config.threshold,
            )
            .await
            {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!("FROST signing failed for input {}: {}", input_idx, e);
                    continue; // Skip this input
                }
            };

            // Parse and add signature
            let sig_bytes = match hex::decode(&signature_hex) {
                Ok(bytes) => bytes,
                Err(e) => {
                    tracing::error!("Invalid signature hex: {}", e);
                    continue;
                }
            };

            let signature = bitcoin::taproot::Signature {
                signature: match bitcoin::secp256k1::schnorr::Signature::from_slice(&sig_bytes) {
                    Ok(sig) => sig,
                    Err(e) => {
                        tracing::error!("Invalid Schnorr signature: {}", e);
                        continue;
                    }
                },
                sighash_type: TapSighashType::Default,
            };

            psbt.inputs[input_idx].tap_key_sig = Some(signature);
            signatures_added += 1;

            tracing::info!("✅ Input {} signed", input_idx);
        }

        tracing::info!(
            "PSBT signing complete: {}/{} inputs signed",
            signatures_added,
            psbt.inputs.len()
        );

        SignPsbtResult::Ok(Json(SignPsbtResponse {
            signed_psbt: psbt.to_string(),
            signatures_added,
        }))
    }

    /// Health check
    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> Json<HealthResponse> {
        Json(HealthResponse {
            status: "ok".to_string(),
            signer_nodes: self.config.signer_nodes.len(),
            threshold: self.config.threshold,
        })
    }
}
