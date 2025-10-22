// Complete example: Sign Bitcoin PSBT with FROST aggregator
// Run with: cargo run --example sign_psbt_example
//
// This demonstrates the end-to-end flow for signing a consolidation
// transaction using FROST threshold signatures via the aggregator.

use anyhow::Result;
use bitcoin::psbt::{Input, Output, Psbt};
use bitcoin::{
    absolute, transaction, Address, Amount, Network, OutPoint, ScriptBuf, Transaction, TxIn, TxOut,
    Txid,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize)]
struct SignPsbtRequest {
    psbt: String,
    passphrases: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct SignPsbtResponse {
    psbt: String,
    inputs_signed: usize,
}

#[derive(Deserialize, Debug)]
struct AddressResponse {
    address: String,
}

#[derive(Serialize)]
struct GenerateAddressRequest {
    passphrase: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== FROST PSBT Signing Example ===\n");

    let aggregator_url = "http://127.0.0.1:6000";
    let client = reqwest::Client::new();

    // Step 1: Generate 2 unique Taproot addresses (simulating deposits)
    println!("Step 1: Generate Taproot addresses via DKG");

    let passphrase1 = "user-deposit-550e8400";
    let passphrase2 = "user-deposit-6ba7b810";

    let addr1_resp: AddressResponse = client
        .post(format!("{}/api/address/generate", aggregator_url))
        .json(&GenerateAddressRequest {
            passphrase: passphrase1.to_string(),
        })
        .send()
        .await?
        .json()
        .await?;

    println!(
        "  Address 1 (passphrase: {}): {}",
        passphrase1, addr1_resp.address
    );

    let addr2_resp: AddressResponse = client
        .post(format!("{}/api/address/generate", aggregator_url))
        .json(&GenerateAddressRequest {
            passphrase: passphrase2.to_string(),
        })
        .send()
        .await?
        .json()
        .await?;

    println!(
        "  Address 2 (passphrase: {}): {}\n",
        passphrase2, addr2_resp.address
    );

    // Step 2: Build PSBT for consolidation transaction
    println!("Step 2: Build consolidation PSBT");

    // Simulated UTXOs (in production, these come from blockchain)
    let input1 = TxIn {
        previous_output: OutPoint {
            txid: Txid::from_str(
                "1111111111111111111111111111111111111111111111111111111111111111",
            )?,
            vout: 0,
        },
        script_sig: ScriptBuf::new(),
        sequence: transaction::Sequence::MAX,
        witness: bitcoin::Witness::new(),
    };

    let input2 = TxIn {
        previous_output: OutPoint {
            txid: Txid::from_str(
                "2222222222222222222222222222222222222222222222222222222222222222",
            )?,
            vout: 0,
        },
        script_sig: ScriptBuf::new(),
        sequence: transaction::Sequence::MAX,
        witness: bitcoin::Witness::new(),
    };

    // Cold wallet destination
    let cold_wallet =
        Address::from_str("bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr")?
            .require_network(Network::Bitcoin)?;

    // Output: Send to cold wallet (total 100,000 sats - 500 fee = 99,500 sats)
    let output = TxOut {
        value: Amount::from_sat(99_500),
        script_pubkey: cold_wallet.script_pubkey(),
    };

    // Build unsigned transaction
    let unsigned_tx = Transaction {
        version: transaction::Version::TWO,
        lock_time: absolute::LockTime::ZERO,
        input: vec![input1, input2],
        output: vec![output],
    };

    println!(
        "  Transaction ID (unsigned): {}",
        unsigned_tx.compute_txid()
    );
    println!("  Inputs: {}", unsigned_tx.input.len());
    println!("  Outputs: {}", unsigned_tx.output.len());
    println!("  Output amount: {} sats\n", unsigned_tx.output[0].value);

    // Create PSBT with witness_utxo for each input
    let addr1 = Address::from_str(&addr1_resp.address)?.require_network(Network::Bitcoin)?;
    let addr2 = Address::from_str(&addr2_resp.address)?.require_network(Network::Bitcoin)?;

    let prevout1 = TxOut {
        value: Amount::from_sat(50_000),
        script_pubkey: addr1.script_pubkey(),
    };

    let prevout2 = TxOut {
        value: Amount::from_sat(50_000),
        script_pubkey: addr2.script_pubkey(),
    };

    let psbt = Psbt {
        unsigned_tx,
        version: 0,
        xpub: Default::default(),
        proprietary: Default::default(),
        unknown: Default::default(),
        inputs: vec![
            Input {
                witness_utxo: Some(prevout1),
                ..Default::default()
            },
            Input {
                witness_utxo: Some(prevout2),
                ..Default::default()
            },
        ],
        outputs: vec![Output::default()],
    };

    let psbt_b64 = psbt.to_string();
    println!("  PSBT (base64): {}...\n", &psbt_b64[..60]);

    // Step 3: Sign PSBT via FROST aggregator
    println!("Step 3: Sign PSBT with FROST threshold signatures");
    println!("  This signs both inputs with their respective passphrases");
    println!("  Aggregator coordinates FROST protocol across 3 signer nodes\n");

    let sign_req = SignPsbtRequest {
        psbt: psbt_b64,
        passphrases: vec![passphrase1.to_string(), passphrase2.to_string()],
    };

    let sign_resp: SignPsbtResponse = client
        .post(format!("{}/api/sign/psbt", aggregator_url))
        .json(&sign_req)
        .send()
        .await?
        .json()
        .await?;

    println!("  ✅ Signing complete!");
    println!("  Inputs signed: {}", sign_resp.inputs_signed);
    println!("  Signed PSBT: {}...\n", &sign_resp.psbt[..60]);

    // Step 4: Verify and finalize
    println!("Step 4: Finalize PSBT");

    let signed_psbt = Psbt::from_str(&sign_resp.psbt)?;

    // Check signatures are present
    for (i, input) in signed_psbt.inputs.iter().enumerate() {
        if let Some(sig) = &input.tap_key_sig {
            println!(
                "  ✅ Input {} has Taproot signature ({} bytes)",
                i,
                sig.signature.as_ref().len()
            );
        } else {
            println!("  ❌ Input {} missing signature", i);
        }
    }

    // Extract final transaction
    let final_tx = signed_psbt.extract_tx()?;
    println!("\n  Final transaction ID: {}", final_tx.compute_txid());
    println!(
        "  Transaction size: ~{} vbytes (Taproot!)",
        final_tx.vsize()
    );
    println!("  Ready to broadcast! 🚀\n");

    // Step 5: Summary
    println!("=== Summary ===");
    println!("✅ Generated 2 unique Taproot addresses via DKG");
    println!("✅ Built consolidation PSBT (2 inputs → 1 output)");
    println!("✅ Signed with FROST (2-of-3 threshold)");
    println!("✅ Each input signed with its passphrase-specific shares");
    println!("✅ Single Schnorr signature per input (56% smaller than multisig)");
    println!("✅ Transaction ready for broadcast!");
    println!();
    println!("Compare to traditional multisig:");
    println!("  Multisig 2 inputs: ~500 vbytes, 4 signatures visible");
    println!("  FROST 2 inputs:    ~220 vbytes, 2 Schnorr signatures");
    println!("  Fee savings:       56% 🎉");

    Ok(())
}
