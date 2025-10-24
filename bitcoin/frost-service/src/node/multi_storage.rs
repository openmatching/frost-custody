/// Multi-curve storage using RocksDB column families
///
/// This storage layer supports multiple elliptic curves in a single database:
/// - secp256k1 (Bitcoin, Ethereum)
/// - Ed25519 (Solana)
use anyhow::{Context, Result};
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use std::sync::Arc;

use crate::curves::{CurveOperations, CurveType};

/// Column family names
const CF_SECP256K1_TR_KEYS: &str = "secp256k1_tr_keys"; // Taproot/Schnorr
const CF_SECP256K1_TR_PUBKEYS: &str = "secp256k1_tr_pubkeys";
const CF_SECP256K1_KEYS: &str = "secp256k1_keys"; // ECDSA
const CF_SECP256K1_PUBKEYS: &str = "secp256k1_pubkeys";
const CF_ED25519_KEYS: &str = "ed25519_keys";
const CF_ED25519_PUBKEYS: &str = "ed25519_pubkeys";

/// Multi-curve share storage
pub struct MultiCurveStorage {
    db: Arc<DB>,
}

impl MultiCurveStorage {
    /// Open or create storage with column families
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Define column families
        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_SECP256K1_TR_KEYS, Options::default()),
            ColumnFamilyDescriptor::new(CF_SECP256K1_TR_PUBKEYS, Options::default()),
            ColumnFamilyDescriptor::new(CF_SECP256K1_KEYS, Options::default()),
            ColumnFamilyDescriptor::new(CF_SECP256K1_PUBKEYS, Options::default()),
            ColumnFamilyDescriptor::new(CF_ED25519_KEYS, Options::default()),
            ColumnFamilyDescriptor::new(CF_ED25519_PUBKEYS, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)
            .context("Failed to open RocksDB with column families")?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Get column family names for curve type
    fn cf_names(&self, curve_type: CurveType) -> (&'static str, &'static str) {
        match curve_type {
            CurveType::Secp256k1Taproot => (CF_SECP256K1_TR_KEYS, CF_SECP256K1_TR_PUBKEYS),
            CurveType::Secp256k1Ecdsa => (CF_SECP256K1_KEYS, CF_SECP256K1_PUBKEYS),
            CurveType::Ed25519 => (CF_ED25519_KEYS, CF_ED25519_PUBKEYS),
        }
    }

    /// Store key package for passphrase
    pub fn store_key_package<C: CurveOperations>(
        &self,
        curve_type: CurveType,
        passphrase: &str,
        key_package: &C::KeyPackage,
    ) -> Result<()>
    where
        C::KeyPackage: Serialize,
    {
        let (cf_keys, _) = self.cf_names(curve_type);
        let cf = self
            .db
            .cf_handle(cf_keys)
            .ok_or_else(|| anyhow::anyhow!("Column family {} not found", cf_keys))?;

        let key = format!("keypackage:{}", passphrase);
        let value = serde_json::to_vec(key_package).context("Failed to serialize key package")?;

        self.db
            .put_cf(&cf, key.as_bytes(), value)
            .context("Failed to store key package")?;

        tracing::debug!("Stored key package for passphrase in {:?}", curve_type);
        Ok(())
    }

    /// Retrieve key package for passphrase
    pub fn get_key_package<C: CurveOperations>(
        &self,
        curve_type: CurveType,
        passphrase: &str,
    ) -> Result<Option<C::KeyPackage>>
    where
        C::KeyPackage: DeserializeOwned,
    {
        let (cf_keys, _) = self.cf_names(curve_type);
        let cf = self
            .db
            .cf_handle(cf_keys)
            .ok_or_else(|| anyhow::anyhow!("Column family {} not found", cf_keys))?;

        let key = format!("keypackage:{}", passphrase);

        match self.db.get_cf(&cf, key.as_bytes())? {
            Some(value) => {
                let key_package: C::KeyPackage =
                    serde_json::from_slice(&value).context("Failed to deserialize key package")?;
                Ok(Some(key_package))
            }
            None => Ok(None),
        }
    }

    /// Store public key package for passphrase
    pub fn store_pubkey_package<C: CurveOperations>(
        &self,
        curve_type: CurveType,
        passphrase: &str,
        pubkey_package: &C::PublicKeyPackage,
    ) -> Result<()>
    where
        C::PublicKeyPackage: Serialize,
    {
        let (_, cf_pubkeys) = self.cf_names(curve_type);
        let cf = self
            .db
            .cf_handle(cf_pubkeys)
            .ok_or_else(|| anyhow::anyhow!("Column family {} not found", cf_pubkeys))?;

        let key = format!("pubkeypackage:{}", passphrase);
        let value =
            serde_json::to_vec(pubkey_package).context("Failed to serialize pubkey package")?;

        self.db
            .put_cf(&cf, key.as_bytes(), value)
            .context("Failed to store pubkey package")?;

        Ok(())
    }

    /// Retrieve public key package for passphrase
    pub fn get_pubkey_package<C: CurveOperations>(
        &self,
        curve_type: CurveType,
        passphrase: &str,
    ) -> Result<Option<C::PublicKeyPackage>>
    where
        C::PublicKeyPackage: DeserializeOwned,
    {
        let (_, cf_pubkeys) = self.cf_names(curve_type);
        let cf = self
            .db
            .cf_handle(cf_pubkeys)
            .ok_or_else(|| anyhow::anyhow!("Column family {} not found", cf_pubkeys))?;

        let key = format!("pubkeypackage:{}", passphrase);

        match self.db.get_cf(&cf, key.as_bytes())? {
            Some(value) => {
                let pubkey_package: C::PublicKeyPackage = serde_json::from_slice(&value)
                    .context("Failed to deserialize pubkey package")?;
                Ok(Some(pubkey_package))
            }
            None => Ok(None),
        }
    }

    /// Check if we have shares for this passphrase and curve
    pub fn has_passphrase(&self, curve_type: CurveType, passphrase: &str) -> bool {
        let (cf_keys, _) = self.cf_names(curve_type);
        if let Some(cf) = self.db.cf_handle(cf_keys) {
            let key = format!("keypackage:{}", passphrase);
            self.db.get_cf(&cf, key.as_bytes()).ok().flatten().is_some()
        } else {
            false
        }
    }
}

/// Curve-specific storage wrapper
pub struct CurveStorage<C: CurveOperations> {
    storage: Arc<MultiCurveStorage>,
    curve_type: CurveType,
    _marker: std::marker::PhantomData<C>,
}

impl<C: CurveOperations> CurveStorage<C> {
    pub fn new(storage: Arc<MultiCurveStorage>, curve_type: CurveType) -> Self {
        Self {
            storage,
            curve_type,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn store_key_package(&self, passphrase: &str, key_package: &C::KeyPackage) -> Result<()>
    where
        C::KeyPackage: Serialize,
    {
        self.storage
            .store_key_package::<C>(self.curve_type, passphrase, key_package)
    }

    pub fn get_key_package(&self, passphrase: &str) -> Result<Option<C::KeyPackage>>
    where
        C::KeyPackage: DeserializeOwned,
    {
        self.storage
            .get_key_package::<C>(self.curve_type, passphrase)
    }

    pub fn store_pubkey_package(
        &self,
        passphrase: &str,
        pubkey_package: &C::PublicKeyPackage,
    ) -> Result<()>
    where
        C::PublicKeyPackage: Serialize,
    {
        self.storage
            .store_pubkey_package::<C>(self.curve_type, passphrase, pubkey_package)
    }

    pub fn get_pubkey_package(&self, passphrase: &str) -> Result<Option<C::PublicKeyPackage>>
    where
        C::PublicKeyPackage: DeserializeOwned,
    {
        self.storage
            .get_pubkey_package::<C>(self.curve_type, passphrase)
    }

    pub fn has_passphrase(&self, passphrase: &str) -> bool {
        self.storage.has_passphrase(self.curve_type, passphrase)
    }
}
