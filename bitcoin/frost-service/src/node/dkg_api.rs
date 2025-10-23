//! Unified Signer Node API (Chain-Agnostic)
//!
//! This API provides all signer node functionality:
//! - Public key queries (secp256k1, Ed25519)
//! - DKG protocol (creates threshold keys)
//! - FROST signing protocol (uses threshold keys)
//!
//! All operations are curve-based. Chain logic lives in aggregator.

use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{ApiResponse, Object, OpenApi};
use std::sync::Arc;

use crate::curves::ed25519::Ed25519Operations;
use crate::curves::secp256k1::Secp256k1Operations;
use crate::curves::CurveType;
use crate::node::config::FrostNode;
use crate::node::multi_storage::{CurveStorage, MultiCurveStorage};

pub struct UnifiedApi {
    pub config: Arc<FrostNode>,
    pub storage: Arc<MultiCurveStorage>,
    pub dkg_state: Arc<crate::node::dkg_state::DkgState>,
}

#[derive(Debug, Object)]
pub struct DkgRound1Request {
    pub passphrase: String,
}

#[derive(Debug, Object)]
pub struct DkgRound1Response {
    pub package: String,
    pub node_index: u16,
}

#[derive(Debug, Object, Clone)]
pub struct DkgRound1Package {
    pub node_index: u16,
    pub package: String,
}

#[derive(Debug, Object)]
pub struct DkgRound2Request {
    pub passphrase: String,
    pub round1_packages: Vec<DkgRound1Package>,
}

#[derive(Debug, Object, Clone)]
pub struct DkgPackageEntry {
    pub sender_index: u16,
    pub recipient_index: u16,
    pub package: String,
}

#[derive(Debug, Object)]
pub struct DkgRound2Response {
    pub packages: Vec<DkgPackageEntry>,
}

#[derive(Debug, Object)]
pub struct DkgFinalizeRequest {
    pub passphrase: String,
    pub round1_packages: Vec<DkgRound1Package>,
    pub round2_packages: Vec<DkgPackageEntry>,
}

#[derive(Debug, Object)]
pub struct DkgFinalizeResponse {
    pub success: bool,
    pub pubkey_hex: String,
}

#[derive(Debug, Object)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(ApiResponse)]
pub enum DkgRound1Result {
    #[oai(status = 200)]
    Ok(Json<DkgRound1Response>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

#[derive(ApiResponse)]
pub enum DkgRound2Result {
    #[oai(status = 200)]
    Ok(Json<DkgRound2Response>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

#[derive(ApiResponse)]
pub enum DkgFinalizeResult {
    #[oai(status = 200)]
    Ok(Json<DkgFinalizeResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

#[OpenApi]
impl UnifiedApi {
    // ========================================================================
    // Public Key Queries
    // ========================================================================

    /// Get secp256k1 public key
    #[oai(path = "/api/curve/secp256k1/pubkey", method = "get")]
    async fn get_secp256k1_pubkey(
        &self,
        Query(passphrase): Query<String>,
    ) -> Result<Json<PublicKeyResponse>, ApiError> {
        let curve_storage =
            CurveStorage::<Secp256k1Operations>::new(self.storage.clone(), CurveType::Secp256k1);

        let pubkey_package = curve_storage
            .get_pubkey_package(&passphrase)
            .map_err(|e| {
                ApiError::InternalError(Json(ErrorResponse {
                    error: format!("Storage error: {}", e),
                }))
            })?
            .ok_or_else(|| {
                ApiError::BadRequest(Json(ErrorResponse {
                    error: "Secp256k1 shares not found. Run DKG first.".to_string(),
                }))
            })?;

        let pubkey_bytes = pubkey_package.verifying_key().serialize().map_err(|e| {
            ApiError::InternalError(Json(ErrorResponse {
                error: format!("Failed to serialize pubkey: {:?}", e),
            }))
        })?;

        Ok(Json(PublicKeyResponse {
            curve: "secp256k1".to_string(),
            passphrase,
            public_key: hex::encode(&pubkey_bytes),
        }))
    }

    /// Get Ed25519 public key
    #[oai(path = "/api/curve/ed25519/pubkey", method = "get")]
    async fn get_ed25519_pubkey(
        &self,
        Query(passphrase): Query<String>,
    ) -> Result<Json<PublicKeyResponse>, ApiError> {
        let curve_storage =
            CurveStorage::<Ed25519Operations>::new(self.storage.clone(), CurveType::Ed25519);

        let pubkey_package = curve_storage
            .get_pubkey_package(&passphrase)
            .map_err(|e| {
                ApiError::InternalError(Json(ErrorResponse {
                    error: format!("Storage error: {}", e),
                }))
            })?
            .ok_or_else(|| {
                ApiError::BadRequest(Json(ErrorResponse {
                    error: "Ed25519 shares not found. Run Ed25519 DKG first.".to_string(),
                }))
            })?;

        let pubkey_bytes = pubkey_package.verifying_key().serialize().map_err(|e| {
            ApiError::InternalError(Json(ErrorResponse {
                error: format!("Failed to serialize Ed25519 pubkey: {:?}", e),
            }))
        })?;

        Ok(Json(PublicKeyResponse {
            curve: "ed25519".to_string(),
            passphrase,
            public_key: hex::encode(&pubkey_bytes),
        }))
    }

    // ========================================================================
    // DKG Protocol - Secp256k1
    // ========================================================================

    /// DKG Round 1: Generate secp256k1 commitment
    #[oai(path = "/api/dkg/round1", method = "post")]
    async fn dkg_round1(&self, req: Json<DkgRound1Request>) -> DkgRound1Result {
        let passphrase = req.0.passphrase;

        tracing::info!("DKG Round 1 for passphrase (secp256k1)");

        // Generate round1 package with deterministic RNG
        match crate::node::derivation::dkg_part1(
            &self.config.master_seed,
            &passphrase,
            self.config.node_index,
            self.config.max_signers,
            self.config.min_signers,
        ) {
            Ok((secret_package, package)) => {
                // Store secret package for round 2
                self.dkg_state
                    .store_round1_secret(&passphrase, secret_package);

                // Serialize package for network transmission
                let package_json = serde_json::to_vec(&package).unwrap();

                DkgRound1Result::Ok(Json(DkgRound1Response {
                    package: hex::encode(package_json),
                    node_index: self.config.node_index,
                }))
            }
            Err(e) => {
                tracing::error!("DKG round1 failed: {}", e);
                DkgRound1Result::InternalError(Json(ErrorResponse {
                    error: format!("DKG round1 failed: {}", e),
                }))
            }
        }
    }

    /// DKG Round 2: Process round1 packages and generate round2 packages
    #[oai(path = "/api/dkg/round2", method = "post")]
    async fn dkg_round2(&self, req: Json<DkgRound2Request>) -> DkgRound2Result {
        let req = req.0;

        tracing::info!("DKG Round 2 for passphrase (secp256k1)");

        // Retrieve our round1 secret
        let round1_secret = match self.dkg_state.get_round1_secret(&req.passphrase) {
            Some(s) => s,
            None => {
                return DkgRound2Result::InternalError(Json(ErrorResponse {
                    error: "Round1 secret not found. Must call round1 first.".to_string(),
                }))
            }
        };

        // Parse all round1 packages (excluding our own)
        let mut round1_packages = std::collections::BTreeMap::new();
        for pkg in req.round1_packages {
            if pkg.node_index == self.config.node_index {
                tracing::debug!("Skipping own package (node {})", pkg.node_index);
                continue;
            }

            let pkg_bytes = match hex::decode(&pkg.package) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(
                        "Failed to decode package from node {}: {}",
                        pkg.node_index,
                        e
                    );
                    continue;
                }
            };

            let package: frost_secp256k1_tr::keys::dkg::round1::Package =
                match serde_json::from_slice(&pkg_bytes) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse package from node {}: {}",
                            pkg.node_index,
                            e
                        );
                        continue;
                    }
                };

            let sender_id = match frost_secp256k1_tr::Identifier::try_from(pkg.node_index + 1) {
                Ok(id) => id,
                Err(e) => {
                    tracing::warn!("Invalid node index {}: {:?}", pkg.node_index, e);
                    continue;
                }
            };

            round1_packages.insert(sender_id, package);
        }

        // Run DKG part2
        let (round2_secret, round2_packages) =
            match frost_secp256k1_tr::keys::dkg::part2(round1_secret, &round1_packages) {
                Ok(result) => result,
                Err(e) => {
                    return DkgRound2Result::InternalError(Json(ErrorResponse {
                        error: format!("DKG round2 failed: {:?}", e),
                    }))
                }
            };

        // Store round2 secret for finalize
        self.dkg_state
            .store_round2_secret(&req.passphrase, round2_secret);

        // Convert packages to response format
        let mut response_packages = Vec::new();
        for (recipient_id, package) in round2_packages {
            // Convert FROST identifier back to node index
            // FROST identifiers are 32-byte scalars in big-endian, so small values (1,2,3)
            // are at the END of the byte array, not the beginning!
            let recipient_bytes = recipient_id.serialize();
            let recipient_index = if !recipient_bytes.is_empty() {
                // Read the last byte (identifiers 1, 2, 3 fit in one byte)
                let last_byte = recipient_bytes[recipient_bytes.len() - 1];
                last_byte.saturating_sub(1) as u16 // Convert 1->0, 2->1, 3->2
            } else {
                0
            };

            let package_json = serde_json::to_vec(&package).unwrap();

            response_packages.push(DkgPackageEntry {
                sender_index: self.config.node_index,
                recipient_index,
                package: hex::encode(package_json),
            });
        }

        DkgRound2Result::Ok(Json(DkgRound2Response {
            packages: response_packages,
        }))
    }

    /// DKG Finalize: Complete key generation and store shares
    #[oai(path = "/api/dkg/finalize", method = "post")]
    async fn dkg_finalize(&self, req: Json<DkgFinalizeRequest>) -> DkgFinalizeResult {
        let req = req.0;

        tracing::info!("DKG Finalize for passphrase (secp256k1)");

        // Retrieve round2 secret
        let round2_secret = match self.dkg_state.get_round2_secret(&req.passphrase) {
            Some(s) => s,
            None => {
                return DkgFinalizeResult::BadRequest(Json(ErrorResponse {
                    error: "Round2 secret not found. Must call round1 and round2 first."
                        .to_string(),
                }))
            }
        };

        // Parse round1 packages (skip own package)
        let mut round1_packages = std::collections::BTreeMap::new();
        for pkg in req.round1_packages {
            // Skip own package - we don't include it in DKG
            if pkg.node_index == self.config.node_index {
                tracing::debug!("Skipping own round1 package (node {})", pkg.node_index);
                continue;
            }

            let pkg_bytes = match hex::decode(&pkg.package) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let package: frost_secp256k1_tr::keys::dkg::round1::Package =
                match serde_json::from_slice(&pkg_bytes) {
                    Ok(p) => p,
                    Err(_) => continue,
                };

            let sender_id = match frost_secp256k1_tr::Identifier::try_from(pkg.node_index + 1) {
                Ok(id) => id,
                Err(_) => continue,
            };

            round1_packages.insert(sender_id, package);
        }

        // Parse round2 packages (only ones for us)
        let mut round2_packages = std::collections::BTreeMap::new();
        for pkg in req.round2_packages {
            let pkg_bytes = match hex::decode(&pkg.package) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let package: frost_secp256k1_tr::keys::dkg::round2::Package =
                match serde_json::from_slice(&pkg_bytes) {
                    Ok(p) => p,
                    Err(_) => continue,
                };

            let sender_id = match frost_secp256k1_tr::Identifier::try_from(pkg.sender_index + 1) {
                Ok(id) => id,
                Err(_) => continue,
            };

            round2_packages.insert(sender_id, package);
        }

        // Debug: Log package counts
        tracing::info!(
            "DKG part3: {} round1 packages, {} round2 packages (expected: {} and {})",
            round1_packages.len(),
            round2_packages.len(),
            self.config.max_signers - 1,  // Should be n-1 (exclude self)
            self.config.max_signers - 1   // Should be n-1
        );

        // Run DKG part3 (finalize)
        let (key_package, pubkey_package) = match frost_secp256k1_tr::keys::dkg::part3(
            &round2_secret,
            &round1_packages,
            &round2_packages,
        ) {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("DKG finalize failed: {:?}", e);
                return DkgFinalizeResult::InternalError(Json(ErrorResponse {
                    error: format!("DKG finalize failed: {:?}", e),
                }));
            }
        };

        // Store key packages in multi-curve storage
        let curve_storage =
            CurveStorage::<Secp256k1Operations>::new(self.storage.clone(), CurveType::Secp256k1);

        if let Err(e) = curve_storage.store_key_package(&req.passphrase, &key_package) {
            tracing::error!("Failed to store key package: {}", e);
        }

        if let Err(e) = curve_storage.store_pubkey_package(&req.passphrase, &pubkey_package) {
            tracing::error!("Failed to store pubkey package: {}", e);
        }

        // Return raw public key hex (aggregator derives chain-specific addresses)
        let pubkey_hex = match pubkey_package.verifying_key().serialize() {
            Ok(bytes) => hex::encode(bytes),
            Err(e) => {
                tracing::error!("Failed to serialize pubkey: {:?}", e);
                return DkgFinalizeResult::InternalError(Json(ErrorResponse {
                    error: format!("Failed to serialize pubkey: {:?}", e),
                }));
            }
        };

        tracing::info!("✅ DKG complete, FROST key shares stored");

        DkgFinalizeResult::Ok(Json(DkgFinalizeResponse {
            success: true,
            pubkey_hex,
        }))
    }

    // ========================================================================
    // FROST Signing Endpoints (with passphrase)
    // ========================================================================

    /// FROST Round 1: Generate signing commitments
    #[oai(path = "/api/frost/round1", method = "post")]
    async fn frost_round1(&self, req: Json<FrostRound1Request>) -> FrostRound1Result {
        let req = req.0;

        tracing::info!("FROST Round 1 for passphrase");

        // Decode message
        let _message = match hex::decode(&req.message) {
            Ok(msg) => msg,
            Err(e) => {
                return FrostRound1Result::InternalError(Json(ErrorResponse {
                    error: format!("Invalid message hex: {}", e),
                }))
            }
        };

        // Get key package for this passphrase
        let curve_storage =
            CurveStorage::<Secp256k1Operations>::new(self.storage.clone(), CurveType::Secp256k1);

        let key_package = match curve_storage.get_key_package(&req.passphrase) {
            Ok(Some(pkg)) => pkg,
            Ok(None) => {
                return FrostRound1Result::InternalError(Json(ErrorResponse {
                    error: "Key package not found for passphrase".to_string(),
                }))
            }
            Err(e) => {
                return FrostRound1Result::InternalError(Json(ErrorResponse {
                    error: format!("Storage error: {}", e),
                }))
            }
        };

        // Generate commitments
        let mut rng = rand::thread_rng();
        let (nonces, commitments) =
            frost_secp256k1_tr::round1::commit(key_package.signing_share(), &mut rng);

        // Serialize commitments
        let commitments_json = serde_json::to_vec(&commitments).unwrap();
        let identifier_hex = hex::encode(key_package.identifier().serialize());

        // Serialize and encrypt nonces (bound to message)
        let message_bytes = hex::decode(&req.message).unwrap();
        let nonces_json = serde_json::to_vec(&nonces).unwrap();
        let encrypted_nonces = match super::crypto::encrypt_nonces(
            &nonces_json,
            &message_bytes,
            &self.config.master_seed,
        ) {
            Ok(enc) => enc,
            Err(e) => {
                return FrostRound1Result::InternalError(Json(ErrorResponse {
                    error: format!("Failed to encrypt nonces: {}", e),
                }))
            }
        };

        FrostRound1Result::Ok(Json(FrostRound1Response {
            identifier: identifier_hex,
            commitments: hex::encode(commitments_json),
            encrypted_nonces,
            node_index: self.config.node_index,
        }))
    }

    /// FROST Round 2: Generate signature share
    #[oai(path = "/api/frost/round2", method = "post")]
    async fn frost_round2(&self, req: Json<FrostRound2Request>) -> FrostRound2Result {
        let req = req.0;

        tracing::info!("FROST Round 2 for passphrase");

        // Decode message
        let message = match hex::decode(&req.message) {
            Ok(msg) => msg,
            Err(e) => {
                return FrostRound2Result::BadRequest(Json(ErrorResponse {
                    error: format!("Invalid message hex: {}", e),
                }))
            }
        };

        // Decrypt nonces
        let nonces_json = match super::crypto::decrypt_nonces(
            &req.encrypted_nonces,
            &message,
            &self.config.master_seed,
        ) {
            Ok(json) => json,
            Err(e) => {
                return FrostRound2Result::BadRequest(Json(ErrorResponse {
                    error: format!("Failed to decrypt nonces: {}", e),
                }))
            }
        };

        let nonces: frost_secp256k1_tr::round1::SigningNonces =
            match serde_json::from_slice(&nonces_json) {
                Ok(n) => n,
                Err(e) => {
                    return FrostRound2Result::InternalError(Json(ErrorResponse {
                        error: format!("Failed to deserialize nonces: {}", e),
                    }))
                }
            };

        // Parse commitments
        let mut commitments_map = std::collections::BTreeMap::new();
        for entry in req.all_commitments {
            let id_str = entry.identifier.trim_matches('"');
            let identifier = match frost_secp256k1_tr::Identifier::deserialize(
                &hex::decode(id_str).unwrap_or_default(),
            ) {
                Ok(id) => id,
                Err(_) => continue,
            };

            let comm_json = match hex::decode(&entry.commitments) {
                Ok(j) => j,
                Err(_) => continue,
            };

            let commitments: frost_secp256k1_tr::round1::SigningCommitments =
                match serde_json::from_slice(&comm_json) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

            commitments_map.insert(identifier, commitments);
        }

        // Get key package for this passphrase
        let curve_storage =
            CurveStorage::<Secp256k1Operations>::new(self.storage.clone(), CurveType::Secp256k1);

        let key_package = match curve_storage.get_key_package(&req.passphrase) {
            Ok(Some(pkg)) => pkg,
            Ok(None) => {
                return FrostRound2Result::BadRequest(Json(ErrorResponse {
                    error: "Key package not found for passphrase".to_string(),
                }))
            }
            Err(e) => {
                return FrostRound2Result::InternalError(Json(ErrorResponse {
                    error: format!("Storage error: {}", e),
                }))
            }
        };

        // Create signing package and sign
        let signing_package = frost_secp256k1_tr::SigningPackage::new(commitments_map, &message);
        let signature_share =
            match frost_secp256k1_tr::round2::sign(&signing_package, &nonces, &key_package) {
                Ok(share) => share,
                Err(e) => {
                    return FrostRound2Result::InternalError(Json(ErrorResponse {
                        error: format!("Failed to sign: {:?}", e),
                    }))
                }
            };

        let share_json = serde_json::to_vec(&signature_share).unwrap();
        let identifier_hex = hex::encode(key_package.identifier().serialize());

        FrostRound2Result::Ok(Json(FrostRound2Response {
            identifier: identifier_hex,
            signature_share: hex::encode(share_json),
            node_index: self.config.node_index,
        }))
    }

    /// FROST Aggregate: Combine signature shares into final signature
    #[oai(path = "/api/frost/aggregate", method = "post")]
    async fn frost_aggregate(&self, req: Json<FrostAggregateRequest>) -> FrostAggregateResult {
        let req = req.0;

        tracing::info!("FROST Aggregate for passphrase");

        // Decode message
        let message = match hex::decode(&req.message) {
            Ok(msg) => msg,
            Err(e) => {
                return FrostAggregateResult::BadRequest(Json(ErrorResponse {
                    error: format!("Invalid message hex: {}", e),
                }))
            }
        };

        // Parse commitments
        let mut commitments_map = std::collections::BTreeMap::new();
        for entry in req.all_commitments {
            let id_str = entry.identifier.trim_matches('"');
            let identifier = match frost_secp256k1_tr::Identifier::deserialize(
                &hex::decode(id_str).unwrap_or_default(),
            ) {
                Ok(id) => id,
                Err(_) => continue,
            };

            let comm_json = match hex::decode(&entry.commitments) {
                Ok(j) => j,
                Err(_) => continue,
            };

            let commitments: frost_secp256k1_tr::round1::SigningCommitments =
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
            let identifier = match frost_secp256k1_tr::Identifier::deserialize(
                &hex::decode(id_str).unwrap_or_default(),
            ) {
                Ok(id) => id,
                Err(_) => continue,
            };

            let share_json = match hex::decode(&entry.share) {
                Ok(j) => j,
                Err(_) => continue,
            };

            let share: frost_secp256k1_tr::round2::SignatureShare =
                match serde_json::from_slice(&share_json) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

            shares_map.insert(identifier, share);
        }

        // Get pubkey package for verification
        let curve_storage =
            CurveStorage::<Secp256k1Operations>::new(self.storage.clone(), CurveType::Secp256k1);

        let pubkey_package = match curve_storage.get_pubkey_package(&req.passphrase) {
            Ok(Some(pkg)) => pkg,
            Ok(None) => {
                return FrostAggregateResult::BadRequest(Json(ErrorResponse {
                    error: "Pubkey package not found for passphrase".to_string(),
                }))
            }
            Err(e) => {
                return FrostAggregateResult::InternalError(Json(ErrorResponse {
                    error: format!("Storage error: {}", e),
                }))
            }
        };

        // Aggregate signature
        let signing_package = frost_secp256k1_tr::SigningPackage::new(commitments_map, &message);
        let signature =
            match frost_secp256k1_tr::aggregate(&signing_package, &shares_map, &pubkey_package) {
                Ok(sig) => sig,
                Err(e) => {
                    return FrostAggregateResult::InternalError(Json(ErrorResponse {
                        error: format!("Failed to aggregate: {:?}", e),
                    }))
                }
            };

        // Verify signature
        let verified = pubkey_package
            .verifying_key()
            .verify(&message, &signature)
            .is_ok();

        let sig_bytes = signature
            .serialize()
            .map_err(|e| format!("Failed to serialize signature: {:?}", e))
            .unwrap_or_default();

        FrostAggregateResult::Ok(Json(FrostAggregateResponse {
            signature: hex::encode(&sig_bytes),
            verified,
        }))
    }

    // ========================================================================
    // DKG Protocol - Ed25519 (for Solana)
    // ========================================================================

    /// DKG Round 1: Generate Ed25519 commitment
    #[oai(path = "/api/dkg/ed25519/round1", method = "post")]
    async fn dkg_ed25519_round1(&self, req: Json<DkgRound1Request>) -> DkgRound1Result {
        let passphrase = req.0.passphrase;

        tracing::info!("DKG Round 1 for passphrase (Ed25519)");

        // Generate round1 package with deterministic RNG
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;
        use sha2::{Digest, Sha256};

        let mut seed_material = self.config.master_seed.clone();
        seed_material.extend_from_slice(b"ed25519:");
        seed_material.extend_from_slice(passphrase.as_bytes());
        let seed_hash = Sha256::digest(&seed_material);
        let seed: [u8; 32] = seed_hash.into();
        let mut rng = ChaCha20Rng::from_seed(seed);

        let participant_id = match frost_ed25519::Identifier::try_from(self.config.node_index + 1) {
            Ok(id) => id,
            Err(e) => {
                return DkgRound1Result::InternalError(Json(ErrorResponse {
                    error: format!("Failed to create participant identifier: {:?}", e),
                }))
            }
        };

        match frost_ed25519::keys::dkg::part1(
            participant_id,
            self.config.max_signers,
            self.config.min_signers,
            &mut rng,
        ) {
            Ok((secret_package, package)) => {
                // Store secret package for round 2
                let secret_key = format!("ed25519:r1:{}", passphrase);
                self.dkg_state
                    .generic_secrets
                    .lock()
                    .unwrap()
                    .insert(secret_key, serde_json::to_vec(&secret_package).unwrap());

                let package_json = serde_json::to_vec(&package).unwrap();

                DkgRound1Result::Ok(Json(DkgRound1Response {
                    package: hex::encode(package_json),
                    node_index: self.config.node_index,
                }))
            }
            Err(e) => {
                tracing::error!("Ed25519 DKG round1 failed: {}", e);
                DkgRound1Result::InternalError(Json(ErrorResponse {
                    error: format!("Ed25519 DKG round1 failed: {}", e),
                }))
            }
        }
    }

    /// DKG Finalize: Complete Ed25519 key generation
    #[oai(path = "/api/dkg/ed25519/finalize", method = "post")]
    async fn dkg_ed25519_finalize(&self, req: Json<DkgFinalizeRequest>) -> DkgFinalizeResult {
        let req = req.0;

        tracing::info!("DKG Finalize for passphrase (Ed25519)");

        // Retrieve round2 secret
        let secret_key = format!("ed25519:r2:{}", req.passphrase);
        let round2_secret_bytes =
            match self
                .dkg_state
                .generic_secrets
                .lock()
                .unwrap()
                .get(&secret_key)
            {
                Some(s) => s.clone(),
                None => return DkgFinalizeResult::BadRequest(Json(ErrorResponse {
                    error:
                        "Round2 secret not found for Ed25519. Must call round1 and round2 first."
                            .to_string(),
                })),
            };

        let round2_secret: frost_ed25519::keys::dkg::round2::SecretPackage =
            match serde_json::from_slice(&round2_secret_bytes) {
                Ok(s) => s,
                Err(e) => {
                    return DkgFinalizeResult::InternalError(Json(ErrorResponse {
                        error: format!("Failed to deserialize round2 secret: {}", e),
                    }))
                }
            };

        // Parse packages (simplified - in production handle routing properly)
        let mut round1_packages = std::collections::BTreeMap::new();
        for pkg in req.round1_packages {
            let pkg_bytes = match hex::decode(&pkg.package) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let package: frost_ed25519::keys::dkg::round1::Package =
                match serde_json::from_slice(&pkg_bytes) {
                    Ok(p) => p,
                    Err(_) => continue,
                };

            let sender_id = match frost_ed25519::Identifier::try_from(pkg.node_index + 1) {
                Ok(id) => id,
                Err(_) => continue,
            };

            round1_packages.insert(sender_id, package);
        }

        let mut round2_packages = std::collections::BTreeMap::new();
        for pkg in req.round2_packages {
            let pkg_bytes = match hex::decode(&pkg.package) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let package: frost_ed25519::keys::dkg::round2::Package =
                match serde_json::from_slice(&pkg_bytes) {
                    Ok(p) => p,
                    Err(_) => continue,
                };

            let sender_id = match frost_ed25519::Identifier::try_from(pkg.sender_index + 1) {
                Ok(id) => id,
                Err(_) => continue,
            };

            round2_packages.insert(sender_id, package);
        }

        // Run DKG part3 (finalize)
        let (key_package, pubkey_package) = match frost_ed25519::keys::dkg::part3(
            &round2_secret,
            &round1_packages,
            &round2_packages,
        ) {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Ed25519 DKG finalize failed: {:?}", e);
                return DkgFinalizeResult::InternalError(Json(ErrorResponse {
                    error: format!("Ed25519 DKG finalize failed: {:?}", e),
                }));
            }
        };

        // Store key packages in Ed25519 column family
        let curve_storage =
            CurveStorage::<Ed25519Operations>::new(self.storage.clone(), CurveType::Ed25519);

        if let Err(e) = curve_storage.store_key_package(&req.passphrase, &key_package) {
            tracing::error!("Failed to store Ed25519 key package: {}", e);
        }

        if let Err(e) = curve_storage.store_pubkey_package(&req.passphrase, &pubkey_package) {
            tracing::error!("Failed to store Ed25519 pubkey package: {}", e);
        }

        // Return raw public key
        let pubkey_hex = match pubkey_package.verifying_key().serialize() {
            Ok(bytes) => hex::encode(bytes),
            Err(e) => {
                tracing::error!("Failed to serialize Ed25519 pubkey: {:?}", e);
                return DkgFinalizeResult::InternalError(Json(ErrorResponse {
                    error: format!("Failed to serialize Ed25519 pubkey: {:?}", e),
                }));
            }
        };

        tracing::info!("✅ Ed25519 DKG complete, key shares stored");

        DkgFinalizeResult::Ok(Json(DkgFinalizeResponse {
            success: true,
            pubkey_hex,
        }))
    }

    // ========================================================================
    // FROST Signing Protocol - Ed25519 (for Solana)
    // ========================================================================

    /// FROST Round 1: Generate Ed25519 signing commitments
    #[oai(path = "/api/frost/ed25519/round1", method = "post")]
    async fn frost_ed25519_round1(&self, req: Json<FrostRound1Request>) -> FrostRound1Result {
        let req = req.0;

        tracing::info!("FROST Round 1 for passphrase (Ed25519)");

        let _message = match hex::decode(&req.message) {
            Ok(msg) => msg,
            Err(e) => {
                return FrostRound1Result::InternalError(Json(ErrorResponse {
                    error: format!("Invalid message hex: {}", e),
                }))
            }
        };

        // Get Ed25519 key package
        let curve_storage =
            CurveStorage::<Ed25519Operations>::new(self.storage.clone(), CurveType::Ed25519);

        let key_package = match curve_storage.get_key_package(&req.passphrase) {
            Ok(Some(pkg)) => pkg,
            Ok(None) => {
                return FrostRound1Result::InternalError(Json(ErrorResponse {
                    error: "Ed25519 key package not found for passphrase".to_string(),
                }))
            }
            Err(e) => {
                return FrostRound1Result::InternalError(Json(ErrorResponse {
                    error: format!("Storage error: {}", e),
                }))
            }
        };

        // Generate commitments
        let mut rng = rand::thread_rng();
        let (nonces, commitments) =
            frost_ed25519::round1::commit(key_package.signing_share(), &mut rng);

        let commitments_json = serde_json::to_vec(&commitments).unwrap();
        let identifier_hex = hex::encode(key_package.identifier().serialize());

        // Encrypt nonces
        let message_bytes = hex::decode(&req.message).unwrap();
        let nonces_json = serde_json::to_vec(&nonces).unwrap();
        let encrypted_nonces = match super::crypto::encrypt_nonces(
            &nonces_json,
            &message_bytes,
            &self.config.master_seed,
        ) {
            Ok(enc) => enc,
            Err(e) => {
                return FrostRound1Result::InternalError(Json(ErrorResponse {
                    error: format!("Failed to encrypt Ed25519 nonces: {}", e),
                }))
            }
        };

        FrostRound1Result::Ok(Json(FrostRound1Response {
            identifier: identifier_hex,
            commitments: hex::encode(commitments_json),
            encrypted_nonces,
            node_index: self.config.node_index,
        }))
    }

    /// FROST Round 2: Generate Ed25519 signature share
    #[oai(path = "/api/frost/ed25519/round2", method = "post")]
    async fn frost_ed25519_round2(&self, req: Json<FrostRound2Request>) -> FrostRound2Result {
        let req = req.0;

        tracing::info!("FROST Round 2 for passphrase (Ed25519)");

        let message = match hex::decode(&req.message) {
            Ok(msg) => msg,
            Err(e) => {
                return FrostRound2Result::BadRequest(Json(ErrorResponse {
                    error: format!("Invalid message hex: {}", e),
                }))
            }
        };

        // Decrypt nonces
        let nonces_json = match super::crypto::decrypt_nonces(
            &req.encrypted_nonces,
            &message,
            &self.config.master_seed,
        ) {
            Ok(json) => json,
            Err(e) => {
                return FrostRound2Result::BadRequest(Json(ErrorResponse {
                    error: format!("Failed to decrypt Ed25519 nonces: {}", e),
                }))
            }
        };

        let nonces: frost_ed25519::round1::SigningNonces =
            match serde_json::from_slice(&nonces_json) {
                Ok(n) => n,
                Err(e) => {
                    return FrostRound2Result::InternalError(Json(ErrorResponse {
                        error: format!("Failed to deserialize Ed25519 nonces: {}", e),
                    }))
                }
            };

        // Parse commitments
        let mut commitments_map = std::collections::BTreeMap::new();
        for entry in req.all_commitments {
            let id_str = entry.identifier.trim_matches('"');
            let identifier = match frost_ed25519::Identifier::deserialize(
                &hex::decode(id_str).unwrap_or_default(),
            ) {
                Ok(id) => id,
                Err(_) => continue,
            };

            let comm_json = match hex::decode(&entry.commitments) {
                Ok(j) => j,
                Err(_) => continue,
            };

            let commitments: frost_ed25519::round1::SigningCommitments =
                match serde_json::from_slice(&comm_json) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

            commitments_map.insert(identifier, commitments);
        }

        // Get Ed25519 key package
        let curve_storage =
            CurveStorage::<Ed25519Operations>::new(self.storage.clone(), CurveType::Ed25519);

        let key_package = match curve_storage.get_key_package(&req.passphrase) {
            Ok(Some(pkg)) => pkg,
            Ok(None) => {
                return FrostRound2Result::BadRequest(Json(ErrorResponse {
                    error: "Ed25519 key package not found for passphrase".to_string(),
                }))
            }
            Err(e) => {
                return FrostRound2Result::InternalError(Json(ErrorResponse {
                    error: format!("Storage error: {}", e),
                }))
            }
        };

        // Create signing package and sign
        let signing_package = frost_ed25519::SigningPackage::new(commitments_map, &message);
        let signature_share =
            match frost_ed25519::round2::sign(&signing_package, &nonces, &key_package) {
                Ok(share) => share,
                Err(e) => {
                    return FrostRound2Result::InternalError(Json(ErrorResponse {
                        error: format!("Failed to sign with Ed25519: {:?}", e),
                    }))
                }
            };

        let share_json = serde_json::to_vec(&signature_share).unwrap();
        let identifier_hex = hex::encode(key_package.identifier().serialize());

        FrostRound2Result::Ok(Json(FrostRound2Response {
            identifier: identifier_hex,
            signature_share: hex::encode(share_json),
            node_index: self.config.node_index,
        }))
    }

    /// FROST Aggregate: Combine Ed25519 signature shares
    #[oai(path = "/api/frost/ed25519/aggregate", method = "post")]
    async fn frost_ed25519_aggregate(
        &self,
        req: Json<FrostAggregateRequest>,
    ) -> FrostAggregateResult {
        let req = req.0;

        tracing::info!("FROST Aggregate for passphrase (Ed25519)");

        let message = match hex::decode(&req.message) {
            Ok(msg) => msg,
            Err(e) => {
                return FrostAggregateResult::BadRequest(Json(ErrorResponse {
                    error: format!("Invalid message hex: {}", e),
                }))
            }
        };

        // Parse commitments
        let mut commitments_map = std::collections::BTreeMap::new();
        for entry in req.all_commitments {
            let id_str = entry.identifier.trim_matches('"');
            let identifier = match frost_ed25519::Identifier::deserialize(
                &hex::decode(id_str).unwrap_or_default(),
            ) {
                Ok(id) => id,
                Err(_) => continue,
            };

            let comm_json = match hex::decode(&entry.commitments) {
                Ok(j) => j,
                Err(_) => continue,
            };

            let commitments: frost_ed25519::round1::SigningCommitments =
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
            let identifier = match frost_ed25519::Identifier::deserialize(
                &hex::decode(id_str).unwrap_or_default(),
            ) {
                Ok(id) => id,
                Err(_) => continue,
            };

            let share_json = match hex::decode(&entry.share) {
                Ok(j) => j,
                Err(_) => continue,
            };

            let share: frost_ed25519::round2::SignatureShare =
                match serde_json::from_slice(&share_json) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

            shares_map.insert(identifier, share);
        }

        // Get pubkey package
        let curve_storage =
            CurveStorage::<Ed25519Operations>::new(self.storage.clone(), CurveType::Ed25519);

        let pubkey_package = match curve_storage.get_pubkey_package(&req.passphrase) {
            Ok(Some(pkg)) => pkg,
            Ok(None) => {
                return FrostAggregateResult::BadRequest(Json(ErrorResponse {
                    error: "Ed25519 pubkey package not found for passphrase".to_string(),
                }))
            }
            Err(e) => {
                return FrostAggregateResult::InternalError(Json(ErrorResponse {
                    error: format!("Storage error: {}", e),
                }))
            }
        };

        // Aggregate signature
        let signing_package = frost_ed25519::SigningPackage::new(commitments_map, &message);
        let signature =
            match frost_ed25519::aggregate(&signing_package, &shares_map, &pubkey_package) {
                Ok(sig) => sig,
                Err(e) => {
                    return FrostAggregateResult::InternalError(Json(ErrorResponse {
                        error: format!("Failed to aggregate Ed25519: {:?}", e),
                    }))
                }
            };

        // Verify signature
        let verified = pubkey_package
            .verifying_key()
            .verify(&message, &signature)
            .is_ok();

        let sig_bytes = signature
            .serialize()
            .map_err(|e| format!("Failed to serialize Ed25519 signature: {:?}", e))
            .unwrap_or_default();

        FrostAggregateResult::Ok(Json(FrostAggregateResponse {
            signature: hex::encode(&sig_bytes),
            verified,
        }))
    }

    // ========================================================================
    // Health Check
    // ========================================================================

    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> Json<HealthResponse> {
        Json(HealthResponse {
            status: "ok".to_string(),
            node_index: self.config.node_index,
            supported_curves: vec!["secp256k1".to_string(), "ed25519".to_string()],
        })
    }
}

// ============================================================================
// Public Key Query Types
// ============================================================================

#[derive(Debug, Object)]
pub struct PublicKeyResponse {
    pub curve: String,
    pub passphrase: String,
    pub public_key: String,
}

#[derive(Debug, Object)]
pub struct HealthResponse {
    pub status: String,
    pub node_index: u16,
    pub supported_curves: Vec<String>,
}

#[derive(Debug, ApiResponse)]
enum ApiError {
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

// ============================================================================
// FROST Signing Request/Response Types
// ============================================================================

#[derive(Debug, Object)]
pub struct FrostRound1Request {
    pub passphrase: String,
    pub message: String,
}

#[derive(Debug, Object)]
pub struct FrostRound1Response {
    pub identifier: String,
    pub commitments: String,
    pub encrypted_nonces: String,
    pub node_index: u16,
}

#[derive(Debug, Object)]
pub struct FrostRound2Request {
    pub passphrase: String,
    pub message: String,
    pub encrypted_nonces: String,
    pub all_commitments: Vec<FrostCommitmentEntry>,
}

#[derive(Debug, Object, Clone)]
pub struct FrostCommitmentEntry {
    pub identifier: String,
    pub commitments: String,
}

#[derive(Debug, Object)]
pub struct FrostRound2Response {
    pub identifier: String,
    pub signature_share: String,
    pub node_index: u16,
}

#[derive(Debug, Object)]
pub struct FrostAggregateRequest {
    pub passphrase: String,
    pub message: String,
    pub all_commitments: Vec<FrostCommitmentEntry>,
    pub signature_shares: Vec<FrostSignatureShareEntry>,
}

#[derive(Debug, Object, Clone)]
pub struct FrostSignatureShareEntry {
    pub identifier: String,
    pub share: String,
}

#[derive(Debug, Object)]
pub struct FrostAggregateResponse {
    pub signature: String,
    pub verified: bool,
}

#[derive(ApiResponse)]
pub enum FrostRound1Result {
    #[oai(status = 200)]
    Ok(Json<FrostRound1Response>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

#[derive(ApiResponse)]
pub enum FrostRound2Result {
    #[oai(status = 200)]
    Ok(Json<FrostRound2Response>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

#[derive(ApiResponse)]
pub enum FrostAggregateResult {
    #[oai(status = 200)]
    Ok(Json<FrostAggregateResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}
