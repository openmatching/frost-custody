use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Client for calling signer-node API
pub struct SignerClient {
    base_url: String,
    client: reqwest::blocking::Client,
}

#[derive(Serialize)]
struct SignRequest {
    psbt: String,
    passphrases: Vec<String>,
}

#[derive(Deserialize)]
struct SignResponse {
    psbt: String,
    signed_count: usize,
    #[allow(dead_code)]
    node_index: u8,
}

impl SignerClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Sign PSBT with this signer node
    ///
    /// # Arguments
    /// * `psbt_base64` - Base64-encoded PSBT
    /// * `passphrases` - Passphrases for each input (in order)
    ///
    /// # Returns
    /// Signed PSBT (base64) and count of signatures added
    pub fn sign(&self, psbt_base64: &str, passphrases: &[String]) -> Result<(String, usize)> {
        let req = SignRequest {
            psbt: psbt_base64.to_string(),
            passphrases: passphrases.to_vec(),
        };

        let resp = self
            .client
            .post(format!("{}/api/sign", self.base_url))
            .json(&req)
            .send()
            .context("Failed to send sign request")?;

        if !resp.status().is_success() {
            let error_text = resp.text().unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Signer returned error: {}", error_text);
        }

        let sign_resp: SignResponse = resp.json().context("Failed to parse sign response")?;

        Ok((sign_resp.psbt, sign_resp.signed_count))
    }
}

/// Sign PSBT with 2-of-3 signer nodes
///
/// # Arguments
/// * `psbt_base64` - Base64-encoded PSBT
/// * `passphrases` - Passphrases for each input
/// * `signer_urls` - URLs of signer nodes (need 2 of 3)
///
/// # Returns
/// Fully signed PSBT (base64) ready to finalize
///
/// # Example
/// ```no_run
/// use frost_mpc_client::signer_client::sign_with_threshold;
///
/// let signer_urls = vec![
///     "http://node0:3000".to_string(),
///     "http://node1:3000".to_string(),
/// ];
/// let psbt_base64 = "cHNidP8BAH...";
/// let passphrases = vec!["passphrase1".to_string()];
///
/// let signed_psbt = sign_with_threshold(
///     psbt_base64,
///     &passphrases,
///     &signer_urls
/// ).unwrap();
/// ```
pub fn sign_with_threshold(
    psbt_base64: &str,
    passphrases: &[String],
    signer_urls: &[String],
) -> Result<String> {
    if signer_urls.len() < 2 {
        anyhow::bail!("Need at least 2 signer nodes for 2-of-3 threshold");
    }

    let mut current_psbt = psbt_base64.to_string();
    let mut total_signed = 0;

    // Sign with first 2 nodes (sufficient for 2-of-3)
    for (i, url) in signer_urls.iter().take(2).enumerate() {
        println!("Signing with node {} at {}...", i, url);

        let client = SignerClient::new(url.clone());
        let (signed_psbt, signed_count) = client.sign(&current_psbt, passphrases)?;

        println!("  ✅ Node {} signed {} inputs", i, signed_count);

        current_psbt = signed_psbt;
        total_signed += signed_count;
    }

    println!("\n✅ Total signatures: {}", total_signed);
    println!("✅ PSBT ready to finalize and broadcast");

    Ok(current_psbt)
}
