// Complete example: Build and sign PSBT from CEX backend
// Run with: cargo run --example sign_psbt

use anyhow::{Context, Result};
use bitcoin::bip32::Xpub;
use bitcoin::{Address, Amount, Network, Txid};
use frost_mpc_client::{
    add_witness_scripts, build_consolidation_psbt, derive_multisig_address, psbt_from_base64,
    psbt_to_base64, sign_with_threshold, Utxo,
};
use std::str::FromStr;

fn main() -> Result<()> {
    println!("=== CEX Client - Complete PSBT Signing Example ===\n");

    // Configuration (get from your CEX database/config)
    let xpubs = vec![
        Xpub::from_str("xpub6EkTGi8Kh6bqYpZzFeoANKQh7nH1GiChpb1StmTSoUG3QA1u6yf6dYprGjWiMBKcTEQ1KFDBNDL4sxDh45AiD7EkFC3yeD23Vkf3yzYSwEb")?,
        Xpub::from_str("xpub6EV2WhLpxRVKo6NPRCXniPmFapNhfeUwzuTZDpsvdiGGa8cPaqzLPqmPmtYy53wXG4NcGZErkPVuFaKQnP3DYCHyTvg1mLyf4vttBdqErFG")?,
        Xpub::from_str("xpub6DyBA7T961cEFdmrvapjPHJGS8abivTPJ9ERFkAZKrz7r9p8Vb33BaenC4JnMia3CuX4byLfS79nJh7qHPGHFHTXR5gjvp8J1r76bXBU7Fx")?,
    ];

    let signer_nodes = vec![
        "http://127.0.0.1:3000".to_string(),
        "http://127.0.0.1:3001".to_string(),
    ];

    // Step 1: Generate deposit addresses (simulating user deposits)
    println!("Step 1: Generate deposit addresses for 2 users\n");

    let passphrase1 = "550e8400-e29b-41d4-a716-446655440000".to_string();
    let passphrase2 = "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string();

    let address1 = derive_multisig_address(&xpubs, &passphrase1, Network::Bitcoin)
        .context("Failed to derive address 1")?;
    let address2 = derive_multisig_address(&xpubs, &passphrase2, Network::Bitcoin)
        .context("Failed to derive address 2")?;

    println!("  User 1 deposit address: {}", address1);
    println!("  User 2 deposit address: {}\n", address2);

    // Step 2: Simulate getting UTXOs from database (after users deposit)
    println!("Step 2: Get UTXOs from database\n");

    let utxos = vec![
        Utxo {
            txid: Txid::from_str(
                "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            )?,
            vout: 0,
            amount: Amount::from_sat(100_000),
            address: address1,
            passphrase: passphrase1,
        },
        Utxo {
            txid: Txid::from_str(
                "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
            )?,
            vout: 1,
            amount: Amount::from_sat(200_000),
            address: address2,
            passphrase: passphrase2,
        },
    ];

    println!("  Found {} UTXOs", utxos.len());
    println!(
        "  Total input: {} sats\n",
        utxos.iter().map(|u| u.amount.to_sat()).sum::<u64>()
    );

    // Step 3: Build consolidation PSBT
    println!("Step 3: Build consolidation PSBT\n");

    let destination = Address::from_str("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")?
        .require_network(Network::Bitcoin)?;
    let fee = Amount::from_sat(5_000);

    let (mut psbt, passphrases) = build_consolidation_psbt(utxos, destination, fee)?;

    println!("  Inputs: {}", psbt.inputs.len());
    println!("  Outputs: {}", psbt.unsigned_tx.output.len());
    println!(
        "  Output amount: {} sats",
        psbt.unsigned_tx.output[0].value.to_sat()
    );
    println!("  Fee: {} sats\n", fee.to_sat());

    // Step 4: Add witness scripts (required for multisig signing)
    println!("Step 4: Add witness scripts to PSBT\n");

    add_witness_scripts(&mut psbt, &xpubs, &passphrases)?;

    println!(
        "  ✅ Witness scripts added for {} inputs\n",
        psbt.inputs.len()
    );

    // Step 5: Serialize PSBT
    let psbt_base64 = psbt_to_base64(&psbt);
    println!("Step 5: PSBT serialized\n");
    println!("  Base64 length: {} chars\n", psbt_base64.len());

    // Step 6: Sign with signer nodes
    println!("Step 6: Sign with signer nodes (2-of-3 threshold)\n");

    let signed_psbt = sign_with_threshold(&psbt_base64, &passphrases, &signer_nodes)?;

    // Step 7: Parse signed PSBT
    println!("\nStep 7: Verify signed PSBT\n");

    let final_psbt = psbt_from_base64(&signed_psbt)?;

    // Count signatures
    let sig_count: usize = final_psbt
        .inputs
        .iter()
        .map(|input| input.partial_sigs.len())
        .sum();

    println!("  Total signatures: {}", sig_count);
    println!("  ✅ PSBT ready to finalize\n");

    // Step 8: Finalize (extract final transaction)
    println!("Step 8: Finalize PSBT\n");
    println!("  (In production, call psbt.finalize() and broadcast)\n");

    println!("=== Summary ===");
    println!("✅ Built PSBT locally (no API call)");
    println!("✅ Added witness scripts locally");
    println!("✅ Signed with 2 signer nodes");
    println!("✅ Transaction ready to broadcast");
    println!();
    println!("CEX only called signer API for signing (not address derivation)");
    println!("Address derivation was done locally (faster, offline-capable)");

    Ok(())
}
