use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Low-level client for calling a single frost-signer node
pub struct FrostNodeClient {
    base_url: String,
    client: reqwest::blocking::Client,
}

/// High-level FROST signing client (manages multiple nodes)
pub struct FrostSignerClient {
    node_urls: Vec<String>,
    threshold: usize, // Number of nodes needed (typically 2 for 2-of-3)
    #[allow(dead_code)]
    client: reqwest::blocking::Client,
}

#[derive(Serialize)]
struct Round1Request {
    message: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Round1Response {
    pub identifier: String,
    pub commitments: String,
    pub encrypted_nonces: String,
    #[allow(dead_code)]
    pub node_index: u16,
}

#[derive(Serialize)]
struct Round2Request {
    message: String,
    encrypted_nonces: String,
    all_commitments: Vec<CommitmentEntry>,
}

#[derive(Serialize, Clone)]
pub struct CommitmentEntry {
    pub identifier: String,
    pub commitments: String,
}

#[derive(Deserialize, Debug)]
pub struct Round2Response {
    pub signature_share: String,
    pub identifier: String,
}

#[derive(Serialize)]
struct AggregateRequest {
    message: String,
    all_commitments: Vec<CommitmentEntry>,
    signature_shares: Vec<SignatureShareEntry>,
}

#[derive(Serialize)]
pub struct SignatureShareEntry {
    pub identifier: String,
    pub share: String,
}

#[derive(Deserialize, Debug)]
pub struct AggregateResponse {
    pub signature: String,
    pub verified: bool,
}

impl FrostNodeClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::blocking::Client::new(),
        }
    }

    /// FROST Round 1: Generate commitments
    pub fn round1(&self, message_hex: &str) -> Result<Round1Response> {
        let req = Round1Request {
            message: message_hex.to_string(),
        };

        let resp = self
            .client
            .post(format!("{}/api/frost/round1", self.base_url))
            .json(&req)
            .send()
            .context("Failed to send round1 request")?;

        if !resp.status().is_success() {
            let error_text = resp.text().unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Round1 error: {}", error_text);
        }

        resp.json().context("Failed to parse round1 response")
    }

    /// FROST Round 2: Sign with commitments
    pub fn round2(
        &self,
        message_hex: &str,
        encrypted_nonces: &str,
        all_commitments: Vec<CommitmentEntry>,
    ) -> Result<Round2Response> {
        let req = Round2Request {
            message: message_hex.to_string(),
            encrypted_nonces: encrypted_nonces.to_string(),
            all_commitments,
        };

        let resp = self
            .client
            .post(format!("{}/api/frost/round2", self.base_url))
            .json(&req)
            .send()
            .context("Failed to send round2 request")?;

        if !resp.status().is_success() {
            let error_text = resp.text().unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Round2 error: {}", error_text);
        }

        resp.json().context("Failed to parse round2 response")
    }

    /// FROST Aggregate: Combine signature shares
    pub fn aggregate(
        &self,
        message_hex: &str,
        all_commitments: Vec<CommitmentEntry>,
        signature_shares: Vec<SignatureShareEntry>,
    ) -> Result<AggregateResponse> {
        let req = AggregateRequest {
            message: message_hex.to_string(),
            all_commitments,
            signature_shares,
        };

        let resp = self
            .client
            .post(format!("{}/api/frost/aggregate", self.base_url))
            .json(&req)
            .send()
            .context("Failed to send aggregate request")?;

        if !resp.status().is_success() {
            let error_text = resp.text().unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Aggregate error: {}", error_text);
        }

        resp.json().context("Failed to parse aggregate response")
    }
}

impl FrostSignerClient {
    /// Create new FROST signer client
    ///
    /// # Arguments
    /// * `node_urls` - URLs of FROST signer nodes (typically 3 nodes)
    /// * `threshold` - Number of nodes needed to sign (typically 2 for 2-of-3)
    ///
    /// # Example
    /// ```
    /// use frost_custody_client::frost_client::FrostSignerClient;
    ///
    /// let client = FrostSignerClient::new(
    ///     vec![
    ///         "http://node0:4000".to_string(),
    ///         "http://node1:4000".to_string(),
    ///     ],
    ///     2  // 2-of-3 threshold
    /// );
    /// assert_eq!(client.threshold(), 2);
    /// ```
    pub fn new(node_urls: Vec<String>, threshold: usize) -> Self {
        Self {
            node_urls,
            threshold,
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Get the configured node URLs
    pub fn node_urls(&self) -> &[String] {
        &self.node_urls
    }

    /// Get the threshold
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// Sign a message with FROST threshold protocol
    ///
    /// # Arguments
    /// * `message_hex` - Message to sign (e.g., Bitcoin sighash)
    ///
    /// # Returns
    /// Final Schnorr signature (hex-encoded)
    pub fn sign_message(&self, message_hex: &str) -> Result<String> {
        if self.node_urls.len() < self.threshold {
            anyhow::bail!(
                "Need at least {} FROST nodes, only {} provided",
                self.threshold,
                self.node_urls.len()
            );
        }

        println!(
            "FROST Round 1: Generating commitments from {} nodes",
            self.threshold
        );

        // Round 1: Get commitments from threshold number of nodes
        let mut round1_responses = Vec::new();
        for (i, url) in self.node_urls.iter().take(self.threshold).enumerate() {
            println!("  Calling node {} at {}...", i, url);

            let client = FrostNodeClient::new(url.clone());
            let r1 = client.round1(message_hex)?;

            println!("  ✅ Node {} commitment received", i);
            round1_responses.push(r1);
        }

        // Prepare commitments for round 2
        let all_commitments: Vec<CommitmentEntry> = round1_responses
            .iter()
            .map(|r| CommitmentEntry {
                identifier: r.identifier.clone(),
                commitments: r.commitments.clone(),
            })
            .collect();

        println!("\nFROST Round 2: Generating signature shares");

        // Round 2: Get signature shares
        let mut round2_responses = Vec::new();
        for (i, (url, r1)) in self
            .node_urls
            .iter()
            .take(self.threshold)
            .zip(&round1_responses)
            .enumerate()
        {
            println!("  Calling node {} at {}...", i, url);

            let client = FrostNodeClient::new(url.clone());
            let r2 = client.round2(message_hex, &r1.encrypted_nonces, all_commitments.clone())?;

            println!("  ✅ Node {} signature share received", i);
            round2_responses.push(r2);
        }

        println!("\nFROST Round 3: Aggregating signature");

        // Round 3: Aggregate signature shares (use first node)
        let signature_shares: Vec<SignatureShareEntry> = round2_responses
            .iter()
            .map(|r| SignatureShareEntry {
                identifier: r.identifier.clone(),
                share: r.signature_share.clone(),
            })
            .collect();

        let client = FrostNodeClient::new(self.node_urls[0].clone());
        let aggregate = client.aggregate(message_hex, all_commitments, signature_shares)?;

        println!("  ✅ Final signature generated");
        println!("  ✅ Verified: {}", aggregate.verified);

        Ok(aggregate.signature)
    }

    /// Sign Bitcoin transaction with FROST
    ///
    /// # Arguments
    /// * `tx` - Unsigned Bitcoin transaction
    /// * `prevouts` - Previous outputs being spent
    ///
    /// # Returns
    /// Transaction with Schnorr signatures in witnesses
    pub fn sign_transaction(
        &self,
        mut tx: bitcoin::Transaction,
        prevouts: &[bitcoin::TxOut],
    ) -> Result<bitcoin::Transaction> {
        use bitcoin::hashes::Hash;
        use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};

        if tx.input.len() != prevouts.len() {
            anyhow::bail!("Input count must match prevouts count");
        }

        let prevouts_all = Prevouts::All(prevouts);

        // Compute all sighashes first
        let mut sighashes = Vec::new();
        {
            let mut sighash_cache = SighashCache::new(&tx);
            for i in 0..tx.input.len() {
                let sighash = sighash_cache
                    .taproot_key_spend_signature_hash(i, &prevouts_all, TapSighashType::Default)
                    .context(format!("Failed to compute sighash for input {}", i))?;
                sighashes.push(sighash);
            }
        }

        // Sign each input
        for (i, sighash) in sighashes.iter().enumerate() {
            println!("\nSigning input {}...", i);

            let message_hex = hex::encode(sighash.as_raw_hash().as_byte_array());

            // Sign with FROST protocol
            let signature_hex = self.sign_message(&message_hex)?;

            // Decode signature
            let signature_bytes =
                hex::decode(&signature_hex).context("Failed to decode signature hex")?;

            // Add to witness
            tx.input[i].witness.push(signature_bytes);

            println!("  ✅ Input {} signed", i);
        }

        println!("\n✅ All {} inputs signed with FROST", tx.input.len());

        Ok(tx)
    }
}

/// Legacy function for backward compatibility
/// Prefer using FrostSignerClient for better control
pub fn frost_sign_message(message_hex: &str, frost_urls: &[String]) -> Result<String> {
    let client = FrostSignerClient::new(frost_urls.to_vec(), 2);
    client.sign_message(message_hex)
}

/// Legacy function for backward compatibility  
/// Prefer using FrostSignerClient for better control
pub fn frost_sign_transaction(
    tx: bitcoin::Transaction,
    prevouts: &[bitcoin::TxOut],
    frost_urls: &[String],
) -> Result<bitcoin::Transaction> {
    let client = FrostSignerClient::new(frost_urls.to_vec(), 2);
    client.sign_transaction(tx, prevouts)
}
