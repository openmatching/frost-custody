use anyhow::{Context, Result};
use frost_secp256k1_tr as frost;
use rocksdb::DB;
use std::path::Path;

/// Storage for FROST key packages per passphrase
pub struct ShareStorage {
    db: DB,
}

impl ShareStorage {
    /// Open or create storage
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = DB::open_default(path).context("Failed to open RocksDB")?;
        Ok(Self { db })
    }

    /// Store key package for passphrase
    pub fn store_key_package(
        &self,
        passphrase: &str,
        key_package: &frost::keys::KeyPackage,
    ) -> Result<()> {
        let key = format!("keypackage:{}", passphrase);
        let value = serde_json::to_vec(key_package).context("Failed to serialize key package")?;

        self.db
            .put(key.as_bytes(), value)
            .context("Failed to store key package")?;

        tracing::debug!("Stored key package for passphrase");
        Ok(())
    }

    /// Retrieve key package for passphrase
    pub fn get_key_package(&self, passphrase: &str) -> Result<Option<frost::keys::KeyPackage>> {
        let key = format!("keypackage:{}", passphrase);

        match self.db.get(key.as_bytes())? {
            Some(value) => {
                let key_package: frost::keys::KeyPackage =
                    serde_json::from_slice(&value).context("Failed to deserialize key package")?;
                Ok(Some(key_package))
            }
            None => Ok(None),
        }
    }

    /// Store public key package for passphrase
    pub fn store_pubkey_package(
        &self,
        passphrase: &str,
        pubkey_package: &frost::keys::PublicKeyPackage,
    ) -> Result<()> {
        let key = format!("pubkeypackage:{}", passphrase);
        let value =
            serde_json::to_vec(pubkey_package).context("Failed to serialize pubkey package")?;

        self.db
            .put(key.as_bytes(), value)
            .context("Failed to store pubkey package")?;

        Ok(())
    }

    /// Retrieve public key package for passphrase
    pub fn get_pubkey_package(
        &self,
        passphrase: &str,
    ) -> Result<Option<frost::keys::PublicKeyPackage>> {
        let key = format!("pubkeypackage:{}", passphrase);

        match self.db.get(key.as_bytes())? {
            Some(value) => {
                let pubkey_package: frost::keys::PublicKeyPackage = serde_json::from_slice(&value)
                    .context("Failed to deserialize pubkey package")?;
                Ok(Some(pubkey_package))
            }
            None => Ok(None),
        }
    }

    /// Check if we have shares for this passphrase
    #[allow(dead_code)]
    pub fn has_passphrase(&self, passphrase: &str) -> bool {
        let key = format!("keypackage:{}", passphrase);
        self.db.get(key.as_bytes()).ok().flatten().is_some()
    }
}
