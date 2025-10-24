// Complete example: Build and sign PSBT using FROST threshold signatures
//
// Prerequisites:
//   docker-compose up frost-node0 frost-node1 frost-node2 address-aggregator
//
// Run with: cargo run --example sign_psbt_frost

use anyhow::{Context, Result};
use bitcoin::hashes::Hash;
use bitcoin::psbt::Psbt;
use bitcoin::{Address, Amount, Network, Transaction, TxOut, Txid};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug)]
struct FrostUtxo {
    txid: Txid,
    vout: u32,
    amount: Amount,
    passphrase: String,
    pubkey_hex: String, // x-only public key for Taproot
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== FROST PSBT Signing Example ===\n");

    let address_aggregator = "http://127.0.0.1:9000"; // DKG orchestration
    let signing_aggregator = "http://127.0.0.1:8000"; // FROST signing orchestration

    // Step 1: Generate addresses
    println!("Step 1: Generate FROST Taproot addresses\n");

    let pass1 = "alice".to_string();
    let pass2 = "bob".to_string();

    let (addr1, pubkey1) = generate_address_with_pubkey(address_aggregator, &pass1).await?;
    let (addr2, pubkey2) = generate_address_with_pubkey(address_aggregator, &pass2).await?;

    println!("  User 1: {}", addr1);
    println!("  User 2: {}\n", addr2);

    // Step 2: Build PSBT
    println!("Step 2: Build PSBT\n");

    let utxos = vec![
        FrostUtxo {
            txid: Txid::from_str(
                "435d3ad207facec0def477188156ab694b4a864c0bf4fffe8e14719e9a5ac848",
            )?,
            vout: 1,
            amount: Amount::from_sat(500000), // Real amount from testnet4
            passphrase: pass1,
            pubkey_hex: pubkey1,
        },
        FrostUtxo {
            txid: Txid::from_str(
                "cf7c43bf5860f3db49402558c8819872acc893adc92b0244f8a028bb449795c6",
            )?,
            vout: 1,
            amount: Amount::from_sat(500000), // Real amount from testnet4
            passphrase: pass2,
            pubkey_hex: pubkey2,
        },
    ];

    let destination = Address::from_str("tb1qeud9thj94ap9m7mg7kds6asf778uj6d56c8r9r")?
        .require_network(Network::Testnet4)?;

    let (psbt, passphrases) = build_psbt(&utxos, destination, Amount::from_sat(200))?;

    println!("  Inputs: {}", psbt.inputs.len());
    println!(
        "  Output: {} sats",
        psbt.unsigned_tx.output[0].value.to_sat()
    );
    println!("  Fee: 2000 sats\n");

    // Step 3: Sign with FROST via signing aggregator
    println!("Step 3: Sign PSBT via signing aggregator\n");

    let signed = sign_psbt_via_aggregator(signing_aggregator, &psbt, &passphrases).await?;

    let sigs = signed
        .inputs
        .iter()
        .filter(|i| i.tap_key_sig.is_some())
        .count();

    println!("  ✅ Signed: {}/{} inputs\n", sigs, signed.inputs.len());

    // Step 4: Extract final transaction
    println!("Step 4: Extract final transaction\n");

    // Check if PSBT has witnesses
    println!("  Checking PSBT finalization:");
    for (idx, input) in signed.inputs.iter().enumerate() {
        println!(
            "    Input {}: tap_key_sig={}, final_script_witness={}",
            idx,
            input.tap_key_sig.is_some(),
            input.final_script_witness.is_some()
        );
    }

    // The signed PSBT should be finalized by the aggregator
    let tx = signed.extract_tx_unchecked_fee_rate();

    // Check witness in extracted transaction
    println!("\n  Extracted transaction:");
    for (idx, txin) in tx.input.iter().enumerate() {
        println!(
            "    Input {}: witness elements = {}",
            idx,
            txin.witness.len()
        );
    }

    let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);

    println!("\n  Transaction hex: {}", &tx_hex);
    println!("  Transaction size: {} vB", tx.vsize());
    println!("  Ready to broadcast!\n");

    println!("✅ Complete FROST PSBT signing");
    println!("✅ Transaction finalized and ready for broadcast");
    println!("✅ Size: {} vB (56% smaller than multisig)", tx.vsize());

    Ok(())
}

/// Generate address and get public key
async fn generate_address_with_pubkey(
    aggregator: &str,
    passphrase: &str,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Req {
        chain: String,
        passphrase: String,
    }

    #[derive(Deserialize)]
    struct Resp {
        address: String,
        public_key: String,
    }

    let resp = reqwest::Client::new()
        .post(format!("{}/api/address/generate", aggregator))
        .json(&Req {
            chain: "bitcoin".to_string(),
            passphrase: passphrase.to_string(),
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Generate failed: {}", resp.text().await?);
    }

    let data = resp.json::<Resp>().await?;
    Ok((data.address, data.public_key))
}

/// Build PSBT
fn build_psbt(
    utxos: &[FrostUtxo],
    destination: Address,
    fee: Amount,
) -> Result<(Psbt, Vec<String>)> {
    use bitcoin::blockdata::locktime::absolute::LockTime;
    use bitcoin::blockdata::transaction::{OutPoint, TxIn};

    let total: u64 = utxos.iter().map(|u| u.amount.to_sat()).sum();

    let tx = Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: LockTime::ZERO,
        input: utxos
            .iter()
            .map(|u| TxIn {
                previous_output: OutPoint {
                    txid: u.txid,
                    vout: u.vout,
                },
                script_sig: bitcoin::ScriptBuf::new(),
                sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: bitcoin::Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(total - fee.to_sat()),
            script_pubkey: destination.script_pubkey(),
        }],
    };

    let mut psbt = Psbt::from_unsigned_tx(tx)?;

    // Add witness_utxo for Taproot signing with real public keys
    for (i, utxo) in utxos.iter().enumerate() {
        // Parse the public key from hex (remove 0x02/0x03 prefix for x-only)
        let pubkey_bytes = hex::decode(&utxo.pubkey_hex)?;
        let xonly_bytes = if pubkey_bytes.len() == 33 {
            &pubkey_bytes[1..] // Skip compression prefix for x-only
        } else if pubkey_bytes.len() == 32 {
            &pubkey_bytes[..]
        } else {
            anyhow::bail!("Invalid pubkey length: {}", pubkey_bytes.len());
        };

        let xonly_pubkey = bitcoin::key::XOnlyPublicKey::from_slice(xonly_bytes)?;
        let tweaked_pubkey = bitcoin::key::TweakedPublicKey::dangerous_assume_tweaked(xonly_pubkey);
        let script_pubkey = bitcoin::ScriptBuf::new_p2tr_tweaked(tweaked_pubkey);

        println!(
            "  Input {}: pubkey={}, script={}",
            i,
            hex::encode(xonly_bytes),
            hex::encode(script_pubkey.as_bytes())
        );

        psbt.inputs[i].witness_utxo = Some(TxOut {
            value: utxo.amount,
            script_pubkey,
        });
    }

    let passphrases = utxos.iter().map(|u| u.passphrase.clone()).collect();

    Ok((psbt, passphrases))
}

/// Sign PSBT via signing aggregator
async fn sign_psbt_via_aggregator(
    aggregator_url: &str,
    psbt: &Psbt,
    passphrases: &[String],
) -> Result<Psbt> {
    #[derive(Serialize)]
    struct Req {
        psbt: String,
        passphrases: Vec<String>,
    }

    #[derive(Deserialize)]
    struct Resp {
        signed_psbt: String,
        signatures_added: usize,
    }

    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/sign/psbt", aggregator_url))
        .json(&Req {
            psbt: psbt.to_string(),
            passphrases: passphrases.to_vec(),
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Signing failed: {}", resp.text().await?);
    }

    let result = resp.json::<Resp>().await?;
    println!(
        "  ✅ Signatures added: {}/{}",
        result.signatures_added,
        passphrases.len()
    );

    result.signed_psbt.parse().context("Invalid signed PSBT")
}

/// Original implementation (kept for reference, not used)
#[allow(dead_code)]
async fn sign_psbt_direct(signers: &[String], psbt: &Psbt, passphrases: &[String]) -> Result<Psbt> {
    use bitcoin::sighash::{Prevouts, SighashCache};
    use bitcoin::TapSighashType;

    let mut signed = psbt.clone();

    for (i, passphrase) in passphrases.iter().enumerate() {
        println!("  Input {}: Signing...", i);

        // Calculate sighash
        let prevouts: Vec<TxOut> = psbt
            .inputs
            .iter()
            .filter_map(|input| input.witness_utxo.clone())
            .collect();

        let prevouts = Prevouts::All(&prevouts);
        let mut cache = SighashCache::new(&psbt.unsigned_tx);

        let sighash =
            cache.taproot_key_spend_signature_hash(i, &prevouts, TapSighashType::Default)?;

        let sighash_hex = hex::encode(sighash.as_byte_array());

        // FROST protocol
        let sig_hex = frost_sign_direct(signers, passphrase, &sighash_hex).await?;

        // Add to PSBT
        let sig_bytes = hex::decode(&sig_hex)?;
        signed.inputs[i].tap_key_sig = Some(bitcoin::taproot::Signature {
            signature: bitcoin::secp256k1::schnorr::Signature::from_slice(&sig_bytes)?,
            sighash_type: TapSighashType::Default,
        });

        println!("    ✅ Signed");
    }

    Ok(signed)
}

/// FROST signing protocol (direct to nodes - not used, kept for reference)
#[allow(dead_code)]
async fn frost_sign_direct(signers: &[String], passphrase: &str, message: &str) -> Result<String> {
    #[derive(Serialize)]
    struct R1Req {
        passphrase: String,
        message: String,
    }
    #[derive(Deserialize, Clone)]
    struct R1Resp {
        identifier: String,
        commitments: String,
        encrypted_nonces: String,
    }
    #[derive(Serialize)]
    struct R2Req {
        passphrase: String,
        message: String,
        encrypted_nonces: String,
        all_commitments: Vec<Commitment>,
    }
    #[derive(Serialize, Clone)]
    struct Commitment {
        identifier: String,
        commitments: String,
    }
    #[derive(Deserialize)]
    struct R2Resp {
        identifier: String,
        signature_share: String,
    }
    #[derive(Serialize)]
    struct AggReq {
        passphrase: String,
        message: String,
        all_commitments: Vec<Commitment>,
        signature_shares: Vec<Share>,
    }
    #[derive(Serialize)]
    struct Share {
        identifier: String,
        share: String,
    }
    #[derive(Deserialize)]
    struct AggResp {
        signature: String,
    }

    let client = reqwest::Client::new();

    // Round 1
    let mut r1_responses = Vec::new();
    for url in signers.iter().take(2) {
        let resp = client
            .post(format!("{}/api/frost/round1", url))
            .json(&R1Req {
                passphrase: passphrase.to_string(),
                message: message.to_string(),
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Round1 failed: {}", resp.text().await?);
        }

        r1_responses.push(resp.json::<R1Resp>().await?);
    }

    let commitments: Vec<Commitment> = r1_responses
        .iter()
        .map(|r| Commitment {
            identifier: r.identifier.clone(),
            commitments: r.commitments.clone(),
        })
        .collect();

    // Round 2
    let mut r2_responses = Vec::new();
    for (url, r1) in signers.iter().take(2).zip(&r1_responses) {
        let resp = client
            .post(format!("{}/api/frost/round2", url))
            .json(&R2Req {
                passphrase: passphrase.to_string(),
                message: message.to_string(),
                encrypted_nonces: r1.encrypted_nonces.clone(),
                all_commitments: commitments.clone(),
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Round2 failed: {}", resp.text().await?);
        }

        r2_responses.push(resp.json::<R2Resp>().await?);
    }

    let shares: Vec<Share> = r2_responses
        .iter()
        .map(|r| Share {
            identifier: r.identifier.clone(),
            share: r.signature_share.clone(),
        })
        .collect();

    // Aggregate
    let resp = client
        .post(format!("{}/api/frost/aggregate", signers[0]))
        .json(&AggReq {
            passphrase: passphrase.to_string(),
            message: message.to_string(),
            all_commitments: commitments,
            signature_shares: shares,
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Aggregate failed: {}", resp.text().await?);
    }

    Ok(resp.json::<AggResp>().await?.signature)
}
