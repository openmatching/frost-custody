use anyhow::Result;
use frost_secp256k1_tr as frost;
use rand::rngs::OsRng;
use std::collections::BTreeMap;
use std::env;

fn main() -> Result<()> {
    println!("=== FROST Key Generation for FROST Custody ===\n");

    // Parse command-line arguments for M-of-N configuration
    let args: Vec<String> = env::args().collect();
    let (min_signers, max_signers) = if args.len() >= 3 {
        let m: u16 = args[1].parse().expect("Invalid min_signers (M)");
        let n: u16 = args[2].parse().expect("Invalid max_signers (N)");
        if m > n {
            eprintln!("Error: M (min_signers) must be ≤ N (max_signers)");
            std::process::exit(1);
        }
        if m < 2 {
            eprintln!("Error: Minimum threshold must be at least 2");
            std::process::exit(1);
        }
        (m, n)
    } else {
        // Default: 2-of-3
        (2, 3)
    };

    println!("Configuration:");
    println!("  Threshold: {}-of-{}", min_signers, max_signers);
    println!(
        "  (Need {} nodes to sign, can tolerate {} failures)",
        min_signers,
        max_signers - min_signers
    );
    println!();

    // Generate keys using trusted dealer
    let rng = OsRng;
    let (shares, _pubkey_package) = frost::keys::generate_with_dealer(
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

    println!("=== FROST KEY PACKAGES ===\n");

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
        println!("master_seed_hex = \"{}\"", key_hex);
        println!("storage_path = \"./data/frost-shares-node{}\"", idx);
        println!("---\n");
    }

    println!("⚠️  IMPORTANT:");
    println!("  1. Each node gets its own master_seed_hex (keep secret!)");
    println!("  2. master_seed_hex is used to deterministically derive FROST shares");
    println!(
        "  3. Backup all {} master seeds separately (BIP39 mnemonics recommended)",
        max_signers
    );
    println!("  4. Shares are stored in RocksDB (cache) and regenerated from master seed");
    println!();
    println!("💡 Note: pubkey_package is no longer in config!");
    println!("   It's stored in RocksDB and regenerated during DKG.");
    println!();
    println!("Usage:");
    println!("  cargo run --bin frost-keygen           # Default 2-of-3");
    println!("  cargo run --bin frost-keygen 3 5       # 3-of-5");
    println!("  cargo run --bin frost-keygen 14 21     # 14-of-21");

    Ok(())
}
