/// Ed25519 curve operations for Solana
use super::CurveOperations;
use anyhow::{anyhow, Context, Result};
use frost_ed25519 as frost;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sha2::{Digest, Sha256};

pub struct Ed25519Operations;

impl Ed25519Operations {
    pub fn new() -> Self {
        Self
    }

    /// Derive deterministic RNG for DKG
    fn derive_dkg_rng(&self, master_seed: &[u8], passphrase: &str) -> ChaCha20Rng {
        let mut seed_material = master_seed.to_vec();
        seed_material.extend_from_slice(b"ed25519:"); // Add curve identifier
        seed_material.extend_from_slice(passphrase.as_bytes());
        let seed_hash = Sha256::digest(&seed_material);
        let seed: [u8; 32] = seed_hash.into();
        ChaCha20Rng::from_seed(seed)
    }
}

impl CurveOperations for Ed25519Operations {
    type KeyPackage = frost::keys::KeyPackage;
    type PublicKeyPackage = frost::keys::PublicKeyPackage;
    type SigningNonces = frost::round1::SigningNonces;
    type SigningCommitments = frost::round1::SigningCommitments;
    type SignatureShare = frost::round2::SignatureShare;
    type Signature = frost::Signature;
    type Round1Secret = frost::keys::dkg::round1::SecretPackage;
    type Round1Package = frost::keys::dkg::round1::Package;
    type Round2Secret = frost::keys::dkg::round2::SecretPackage;
    type Round2Package = frost::keys::dkg::round2::Package;
    type Identifier = frost::Identifier;

    fn dkg_round1(
        &self,
        master_seed: &[u8],
        passphrase: &str,
        node_index: u16,
        max_signers: u16,
        min_signers: u16,
    ) -> Result<(Self::Round1Secret, Self::Round1Package)> {
        let mut rng = self.derive_dkg_rng(master_seed, passphrase);

        let participant_id = frost::Identifier::try_from(node_index + 1)
            .context("Failed to create participant identifier")?;

        let (secret_package, package) =
            frost::keys::dkg::part1(participant_id, max_signers, min_signers, &mut rng)?;

        Ok((secret_package, package))
    }

    fn dkg_round2(
        &self,
        round1_secret: Self::Round1Secret,
        round1_packages: &[Self::Round1Package],
    ) -> Result<(Self::Round2Secret, Self::Round2Package)> {
        // Build map of round1 packages
        let round1_map: std::collections::BTreeMap<_, _> = round1_packages
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let id = frost::Identifier::try_from((i + 1) as u16).ok()?;
                Some((id, p.clone()))
            })
            .collect();

        let (_round2_secret, _round2_packages) =
            frost::keys::dkg::part2(round1_secret, &round1_map)?;

        // Return placeholder - DKG needs proper package routing
        Err(anyhow!(
            "DKG round2 returns multiple packages - use node API directly"
        ))
    }

    fn dkg_finalize(
        &self,
        _round2_secret: Self::Round2Secret,
        _round1_packages: &[Self::Round1Package],
        _round2_packages: &[Self::Round2Package],
    ) -> Result<(Self::KeyPackage, Self::PublicKeyPackage)> {
        // DKG finalization requires proper package routing
        Err(anyhow!(
            "DKG finalize requires proper package routing - use node API directly"
        ))
    }

    fn sign_round1(
        &self,
        key_package: &Self::KeyPackage,
        _message: &[u8],
    ) -> Result<(Self::SigningNonces, Self::SigningCommitments)> {
        let mut rng = rand::thread_rng();
        let (nonces, commitments) = frost::round1::commit(key_package.signing_share(), &mut rng);
        Ok((nonces, commitments))
    }

    fn sign_round2(
        &self,
        key_package: &Self::KeyPackage,
        nonces: Self::SigningNonces,
        message: &[u8],
        commitments: &[Self::SigningCommitments],
    ) -> Result<Self::SignatureShare> {
        let commitment_map: std::collections::BTreeMap<_, _> = commitments
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let id = frost::Identifier::try_from((i + 1) as u16).ok()?;
                Some((id, *c))
            })
            .collect();

        let signing_package = frost::SigningPackage::new(commitment_map, message);

        let signature_share = frost::round2::sign(&signing_package, &nonces, key_package)?;

        Ok(signature_share)
    }

    fn aggregate_signature(
        &self,
        pubkey_package: &Self::PublicKeyPackage,
        message: &[u8],
        commitments: &[Self::SigningCommitments],
        shares: &[Self::SignatureShare],
    ) -> Result<Self::Signature> {
        let commitment_map: std::collections::BTreeMap<_, _> = commitments
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let id = frost::Identifier::try_from((i + 1) as u16).ok()?;
                Some((id, *c))
            })
            .collect();

        let signing_package = frost::SigningPackage::new(commitment_map, message);

        let share_map: std::collections::BTreeMap<_, _> = shares
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let id = frost::Identifier::try_from((i + 1) as u16).ok()?;
                Some((id, *s))
            })
            .collect();

        let signature = frost::aggregate(&signing_package, &share_map, pubkey_package)?;

        Ok(signature)
    }

    fn verify_signature(
        &self,
        pubkey_package: &Self::PublicKeyPackage,
        message: &[u8],
        signature: &Self::Signature,
    ) -> Result<bool> {
        let verifying_key = pubkey_package.verifying_key();
        match verifying_key.verify(message, signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn identifier_to_hex(&self, id: &Self::Identifier) -> String {
        hex::encode(id.serialize())
    }

    fn identifier_from_hex(&self, hex_str: &str) -> Result<Self::Identifier> {
        let bytes = hex::decode(hex_str).context("Invalid hex")?;
        frost::Identifier::deserialize(&bytes).context("Invalid identifier")
    }
}
