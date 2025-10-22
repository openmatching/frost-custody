// Example: Derive addresses in CEX backend without calling signer API
// Run with: cargo run --example derive_address

use anyhow::Result;
use bitcoin::bip32::Xpub;
use bitcoin::Network;
use frost_custody_client::{derive_multisig_address, passphrase_to_derivation_path};
use std::str::FromStr;

fn main() -> Result<()> {
    println!("=== CEX Client - Local Address Derivation ===\n");

    // Example xpubs from signer nodes (get these from /health endpoint)
    let xpubs = vec![
        Xpub::from_str("xpub6EkTGi8Kh6bqYpZzFeoANKQh7nH1GiChpb1StmTSoUG3QA1u6yf6dYprGjWiMBKcTEQ1KFDBNDL4sxDh45AiD7EkFC3yeD23Vkf3yzYSwEb")?,
        Xpub::from_str("xpub6EV2WhLpxRVKo6NPRCXniPmFapNhfeUwzuTZDpsvdiGGa8cPaqzLPqmPmtYy53wXG4NcGZErkPVuFaKQnP3DYCHyTvg1mLyf4vttBdqErFG")?,
        Xpub::from_str("xpub6DyBA7T961cEFdmrvapjPHJGS8abivTPJ9ERFkAZKrz7r9p8Vb33BaenC4JnMia3CuX4byLfS79nJh7qHPGHFHTXR5gjvp8J1r76bXBU7Fx")?,
    ];

    // Example passphrase (in production, use UUID)
    let passphrase = "550e8400-e29b-41d4-a716-446655440000";

    println!("Passphrase: {}", passphrase);

    // Show derivation path
    let path = passphrase_to_derivation_path(passphrase);
    println!("Derivation path: {}", path);
    println!("  (9 levels, each < 2^31, full 256-bit space)\n");

    // Derive address locally (standard BIP32)
    let address = derive_multisig_address(&xpubs, passphrase, Network::Bitcoin)?;

    println!("Derived address: {}", address);
    println!();

    println!("✅ Address derived locally using standard BIP32");
    println!("✅ No API call to signer nodes needed");
    println!("✅ Same result as calling GET /api/address?passphrase=...");
    println!("✅ Full 256-bit keyspace (no birthday paradox)");
    println!();

    // Show how to use in CEX
    println!("=== CEX Integration Example ===\n");
    println!("// When user requests deposit address:");
    println!("let passphrase = Uuid::new_v4().to_string();");
    println!("let address = derive_multisig_address(&xpubs, &passphrase, Network::Bitcoin)?;");
    println!("db.insert(user_id, passphrase, address);");
    println!();
    println!("// NO API call needed! Fast and efficient.");

    Ok(())
}
