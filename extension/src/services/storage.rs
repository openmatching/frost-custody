// Chrome Storage API Integration
// Plaintext storage for MVP (encryption in Phase 2)

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use serde::{Deserialize, Serialize};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["chrome", "storage", "local"])]
    fn get(keys: JsValue) -> js_sys::Promise;
    
    #[wasm_bindgen(js_namespace = ["chrome", "storage", "local"])]
    fn set(items: JsValue) -> js_sys::Promise;
    
    #[wasm_bindgen(js_namespace = ["chrome", "storage", "local"])]
    fn remove(keys: JsValue) -> js_sys::Promise;
    
    #[wasm_bindgen(js_namespace = ["chrome", "storage", "local"])]
    fn clear() -> js_sys::Promise;
}

#[derive(Serialize, Deserialize)]
struct WalletData {
    key_package: String,  // Serialized FROST key package
    group_public_key: String,
    address: String,
    threshold: u16,
    total_participants: u16,
    created_at: i64,
}

pub struct StorageManager;

impl StorageManager {
    pub fn new() -> Self {
        Self
    }

    /// Store key package (plaintext for MVP)
    /// Phase 2: Encrypt with WebAuthn before storing
    pub async fn store_key_package(&self, key_package: &str) -> Result<(), JsValue> {
        log::info!("Storing key package...");

        let wallet_data = WalletData {
            key_package: key_package.to_string(),
            group_public_key: String::new(), // Will be filled
            address: String::new(),
            threshold: 2,
            total_participants: 3,
            created_at: js_sys::Date::now() as i64,
        };

        let data = serde_json::to_string(&wallet_data)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"wallet_data".into(), &data.into())?;

        JsFuture::from(set(obj.into())).await?;
        log::info!("Key package stored successfully");

        Ok(())
    }

    /// Retrieve key package
    pub async fn get_key_package(&self) -> Result<Option<String>, JsValue> {
        log::info!("Retrieving key package...");

        let keys = js_sys::Array::new();
        keys.push(&"wallet_data".into());

        let result = JsFuture::from(get(keys.into())).await?;

        if let Some(wallet_data_str) = js_sys::Reflect::get(&result, &"wallet_data".into())?
            .as_string()
        {
            let wallet_data: WalletData = serde_json::from_str(&wallet_data_str)
                .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

            Ok(Some(wallet_data.key_package))
        } else {
            Ok(None)
        }
    }

    /// Store wallet metadata
    pub async fn store_metadata(
        &self,
        group_public_key: &str,
        address: &str,
        threshold: u16,
    ) -> Result<(), JsValue> {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"group_public_key".into(), &group_public_key.into())?;
        js_sys::Reflect::set(&obj, &"address".into(), &address.into())?;
        js_sys::Reflect::set(&obj, &"threshold".into(), &(threshold as f64).into())?;

        JsFuture::from(set(obj.into())).await?;
        Ok(())
    }

    /// Get wallet address
    pub async fn get_address(&self) -> Result<Option<String>, JsValue> {
        let keys = js_sys::Array::new();
        keys.push(&"address".into());

        let result = JsFuture::from(get(keys.into())).await?;
        Ok(js_sys::Reflect::get(&result, &"address".into())?.as_string())
    }

    /// Clear all storage
    pub async fn clear(&self) -> Result<(), JsValue> {
        log::info!("Clearing storage...");
        JsFuture::from(clear()).await?;
        log::info!("Storage cleared");
        Ok(())
    }

    /// Check if wallet exists
    pub async fn has_wallet(&self) -> Result<bool, JsValue> {
        Ok(self.get_key_package().await?.is_some())
    }
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}

