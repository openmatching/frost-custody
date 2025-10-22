use anyhow::Result;
use frost_secp256k1_tr as frost;
use rand::rngs::OsRng;
use std::collections::BTreeMap;

fn main() -> Result<()> {
    println!("=== FROST Key Generation for Consensus Ring ===\n");

    let max_signers = 3;
    let min_signers = 2;

    println!("Configuration:");
    println!("  Max signers: {}", max_signers);
    println!("  Min signers (threshold): {}", min_signers);
    println!();

    // Generate keys using trusted dealer
    let rng = OsRng;
    let (shares, pubkey_package) = frost::keys::generate_with_dealer(
        max_signers,
        min_signers,
        frost::keys::IdentifierList::Default,
        rng,
    )?;

    println!("Generated {} key shares\n", shares.len());

    // Convert to key packages
    let mut key_packages = BTreeMap::new();
    for (identifier, secret_share) in shares {
        let key_package = frost::keys::KeyPackage::try_from(secret_share)?;
        key_packages.insert(identifier, key_package);
    }

    // Serialize public key package (shared across all nodes)
    let pubkey_json = serde_json::to_vec(&pubkey_package)?;
    let pubkey_hex = hex::encode(&pubkey_json);

    println!("=== GROUP PUBLIC KEY (share with all nodes) ===");
    println!("{}\n", pubkey_hex);

    // Output each key package
    for (idx, (identifier, key_package)) in key_packages.iter().enumerate() {
        println!("=== NODE {} ===", idx);
        println!("Identifier: {:?}", identifier);

        let key_json = serde_json::to_vec(&key_package)?;
        let key_hex = hex::encode(&key_json);

        println!("Key package (SECRET - store in config):");
        println!("{}\n", key_hex);

        println!("Config snippet for node {}:", idx);
        println!("---");
        println!("[frost]");
        println!("node_index = {}", idx);
        println!("min_signers = {}", min_signers);
        println!("max_signers = {}", max_signers);
        println!("key_package_hex = \"{}\"", key_hex);
        println!("pubkey_package_hex = \"{}\"", pubkey_hex);
        println!("---\n");
    }

    println!("⚠️  IMPORTANT:");
    println!("  1. Each node gets its own key_package_hex (keep secret!)");
    println!("  2. All nodes share the same pubkey_package_hex");
    println!("  3. Store securely - losing keys = losing funds");
    println!("  4. Backup all 3 key packages separately");

    Ok(())
}
