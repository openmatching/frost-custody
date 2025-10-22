use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{ApiResponse, Object, OpenApi};
use std::sync::Arc;

use crate::config::FrostNode;

pub struct Api {
    pub config: Arc<FrostNode>,
    pub storage: Arc<crate::storage::ShareStorage>,
    pub dkg_state: Arc<crate::dkg_state::DkgState>,
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
    /// Passphrase for address derivation
    pub passphrase: String,
    /// Message to sign (hex)
    pub message: String,
}

#[derive(Debug, Object)]
pub struct CommitmentsResponse {
    /// Signing commitments (hex)
    pub commitments: String,
    /// Encrypted nonces (return this in round2)
    pub encrypted_nonces: String,
    /// Node index
    pub node_index: u16,
}

#[derive(Debug, Object)]
pub struct SignRequest {
    /// Passphrase for address derivation
    pub passphrase: String,
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
    /// Node index
    pub node_index: u16,
}

#[derive(Debug, Object)]
pub struct AggregateRequest {
    /// Passphrase for address derivation
    pub passphrase: String,
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
    /// Signing mode
    pub mode: String,
}

#[derive(Debug, Object)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
}

// DKG Types
#[derive(Debug, Object)]
pub struct DkgRound1Request {
    /// Passphrase for address derivation
    pub passphrase: String,
}

#[derive(Debug, Object)]
pub struct DkgRound1Response {
    /// Serialized round1 package (hex)
    pub package: String,
    /// Node index
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

#[derive(Debug, Object)]
pub struct DkgRound2Response {
    /// Map of recipient_index → package (hex)
    pub packages: Vec<DkgPackageEntry>,
}

#[derive(Debug, Object, Clone)]
pub struct DkgPackageEntry {
    pub sender_index: u16,    // Who sent this package
    pub recipient_index: u16, // Who it's for
    pub package: String,
}

#[derive(Debug, Object)]
pub struct DkgFinalizeRequest {
    pub passphrase: String,
    pub round1_packages: Vec<DkgRound1Package>,
    pub round2_packages: Vec<DkgPackageEntry>, // Packages for this node
}

#[derive(Debug, Object)]
pub struct DkgFinalizeResponse {
    pub success: bool,
    pub address: String,
}

#[derive(ApiResponse)]
pub enum AddressResult {
    #[oai(status = 200)]
    Ok(Json<AddressResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
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

        match crate::signer::generate_commitments(&self.config, &req.passphrase) {
            Ok((nonces, commitments)) => {
                // Serialize commitments
                let commitments_json = serde_json::to_vec(&commitments).unwrap();

                // Serialize and encrypt nonces (bound to message)
                let nonces_json = serde_json::to_vec(&nonces).unwrap();
                let encrypted_nonces = match crate::crypto::encrypt_nonces(
                    &nonces_json,
                    &message,
                    &self.config.master_seed,
                ) {
                    Ok(enc) => enc,
                    Err(e) => {
                        return CommitmentsResult::InternalError(Json(ErrorResponse {
                            error: format!("Failed to encrypt nonces: {}", e),
                        }))
                    }
                };

                CommitmentsResult::Ok(Json(CommitmentsResponse {
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
            &self.config.master_seed,
        ) {
            Ok(json) => json,
            Err(e) => {
                return SignResult::BadRequest(Json(ErrorResponse {
                    error: format!("Failed to decrypt nonces: {}", e),
                }))
            }
        };

        let nonces: frost_secp256k1_tr::round1::SigningNonces =
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

        // Sign
        match crate::signer::sign_with_commitments(
            &self.config,
            &req.passphrase,
            &message,
            &nonces,
            commitments_map,
        ) {
            Ok(signature_share) => {
                let share_json = serde_json::to_vec(&signature_share).unwrap();

                SignResult::Ok(Json(SignResponse {
                    signature_share: hex::encode(share_json),
                    node_index: self.config.node_index,
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

        // Aggregate
        match crate::signer::aggregate_signature(
            &self.config,
            &req.passphrase,
            &message,
            commitments_map,
            shares_map,
        ) {
            Ok(signature) => {
                // Verify signature
                let verified = crate::signer::verify_signature(
                    &self.config,
                    &req.passphrase,
                    &message,
                    &signature,
                )
                .is_ok();

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

    /// DKG Round 1: Generate commitment
    #[oai(path = "/api/dkg/round1", method = "post")]
    async fn dkg_round1(&self, req: Json<DkgRound1Request>) -> DkgRound1Result {
        let passphrase = req.0.passphrase;

        tracing::info!("DKG Round 1 for passphrase");

        // Generate round1 package with deterministic RNG
        match crate::derivation::dkg_part1(
            &self.config.master_seed,
            &passphrase,
            self.config.node_index,
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

        tracing::info!("DKG Round 2 for passphrase");

        // Retrieve our round1 secret
        let round1_secret = match self.dkg_state.get_round1_secret(&req.passphrase) {
            Some(s) => s,
            None => {
                return DkgRound2Result::InternalError(Json(ErrorResponse {
                    error: "Round1 secret not found. Must call round1 first.".to_string(),
                }))
            }
        };

        // Parse all round1 packages (including our own!)
        // FROST DKG requires ALL packages
        tracing::info!(
            "DKG round2: Received {} round1 packages",
            req.round1_packages.len()
        );

        let mut round1_packages = std::collections::BTreeMap::new();
        for pkg in req.round1_packages {
            // Skip our own package - FROST dkg::part2 expects packages from OTHER participants only
            if pkg.node_index == self.config.node_index {
                tracing::debug!("Skipping own package (node {})", pkg.node_index);
                continue;
            }

            tracing::debug!("Processing package from node {}", pkg.node_index);
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

            // Get sender identifier
            let sender_id =
                match frost_secp256k1_tr::Identifier::try_from((pkg.node_index + 1) as u16) {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::warn!("Invalid node index {}: {:?}", pkg.node_index, e);
                        continue;
                    }
                };

            round1_packages.insert(sender_id, package);
        }

        tracing::info!(
            "DKG round2: Processed {} packages from other nodes (expected 2 for 3-node DKG)",
            round1_packages.len()
        );

        // Run DKG part2
        match frost_secp256k1_tr::keys::dkg::part2(round1_secret, &round1_packages) {
            Ok((round2_secret, round2_packages)) => {
                // Store round2 secret for finalize
                self.dkg_state
                    .store_round2_secret(&req.passphrase, round2_secret);

                // Serialize packages for each recipient
                let mut packages = Vec::new();
                for (recipient_id, package) in round2_packages {
                    // Extract index from identifier
                    let id_bytes = recipient_id.serialize();
                    let recipient_index = u16::from_be_bytes([
                        id_bytes[id_bytes.len() - 2],
                        id_bytes[id_bytes.len() - 1],
                    ]) - 1;
                    let package_json = serde_json::to_vec(&package).unwrap();

                    packages.push(DkgPackageEntry {
                        sender_index: self.config.node_index, // We are the sender
                        recipient_index,
                        package: hex::encode(package_json),
                    });
                }

                DkgRound2Result::Ok(Json(DkgRound2Response { packages }))
            }
            Err(e) => {
                tracing::error!("DKG round2 failed: {:?}", e);
                DkgRound2Result::InternalError(Json(ErrorResponse {
                    error: format!("DKG round2 failed: {:?}", e),
                }))
            }
        }
    }

    /// DKG Finalize: Complete DKG and store shares
    #[oai(path = "/api/dkg/finalize", method = "post")]
    async fn dkg_finalize(&self, req: Json<DkgFinalizeRequest>) -> DkgFinalizeResult {
        let req = req.0;

        tracing::info!("DKG Finalize for passphrase");

        // Retrieve our round2 secret
        let round2_secret = match self.dkg_state.remove_round2_secret(&req.passphrase) {
            Some(s) => s,
            None => {
                return DkgFinalizeResult::BadRequest(Json(ErrorResponse {
                    error: "Round2 secret not found. Must call round2 first.".to_string(),
                }))
            }
        };

        // Parse round1 packages from OTHER nodes only
        let mut round1_packages = std::collections::BTreeMap::new();
        for pkg in req.round1_packages {
            // Skip our own package
            if pkg.node_index == self.config.node_index {
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

            let sender_id =
                match frost_secp256k1_tr::Identifier::try_from((pkg.node_index + 1) as u16) {
                    Ok(id) => id,
                    Err(_) => continue,
                };

            round1_packages.insert(sender_id, package);
        }

        tracing::info!(
            "DKG finalize: Processed {} round1 packages from other nodes",
            round1_packages.len()
        );

        // Parse round2 packages (packages sent TO this node from other nodes)
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

            // Use sender_index to identify who sent this package
            let sender_id =
                match frost_secp256k1_tr::Identifier::try_from((pkg.sender_index + 1) as u16) {
                    Ok(id) => id,
                    Err(_) => continue,
                };

            round2_packages.insert(sender_id, package);
        }

        tracing::info!(
            "DKG finalize: Processed {} round2 packages",
            round2_packages.len()
        );

        // Run DKG part3 (finalize)
        match frost_secp256k1_tr::keys::dkg::part3(
            &round2_secret,
            &round1_packages,
            &round2_packages,
        ) {
            Ok((key_package, pubkey_package)) => {
                // Store in persistent cache
                if let Err(e) = self
                    .storage
                    .store_key_package(&req.passphrase, &key_package)
                {
                    tracing::error!("Failed to store key package: {}", e);
                }
                if let Err(e) = self
                    .storage
                    .store_pubkey_package(&req.passphrase, &pubkey_package)
                {
                    tracing::error!("Failed to store pubkey package: {}", e);
                }

                // Derive Taproot address
                let address = match self.derive_taproot_address(&pubkey_package) {
                    Ok(addr) => addr,
                    Err(e) => {
                        tracing::error!("Failed to derive address: {}", e);
                        return DkgFinalizeResult::InternalError(Json(ErrorResponse {
                            error: format!("Failed to derive address: {}", e),
                        }));
                    }
                };

                tracing::info!("✅ DKG complete, shares stored for passphrase");

                DkgFinalizeResult::Ok(Json(DkgFinalizeResponse {
                    success: true,
                    address,
                }))
            }
            Err(e) => {
                tracing::error!("DKG finalize failed: {:?}", e);
                DkgFinalizeResult::InternalError(Json(ErrorResponse {
                    error: format!("DKG finalize failed: {:?}", e),
                }))
            }
        }
    }

    fn derive_taproot_address(
        &self,
        pubkey_package: &frost_secp256k1_tr::keys::PublicKeyPackage,
    ) -> anyhow::Result<String> {
        let group_pubkey = pubkey_package.verifying_key();
        let group_pubkey_bytes = group_pubkey
            .serialize()
            .map_err(|e| anyhow::anyhow!("Failed to serialize pubkey: {:?}", e))?;

        let secp_pubkey = bitcoin::secp256k1::PublicKey::from_slice(&group_pubkey_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to parse pubkey: {:?}", e))?;

        let pubkey_bytes_full = secp_pubkey.serialize();
        let x_only = bitcoin::key::XOnlyPublicKey::from_slice(&pubkey_bytes_full[1..33])
            .map_err(|e| anyhow::anyhow!("Failed to create x-only: {:?}", e))?;

        let address = bitcoin::Address::p2tr_tweaked(
            bitcoin::key::TweakedPublicKey::dangerous_assume_tweaked(x_only),
            self.config.network,
        );

        Ok(address.to_string())
    }

    /// Health check
    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> Json<HealthResponse> {
        Json(HealthResponse {
            status: "ok".to_string(),
            node_index: self.config.node_index,
            mode: "FROST threshold signature".to_string(),
        })
    }
}
