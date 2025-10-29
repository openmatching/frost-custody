use anyhow::Result;
use std::env;

fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║     FROST HSM Key Generation - Setup Guide               ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Parse threshold configuration from args
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

    println!("Target Configuration:");
    println!("  Threshold: {}-of-{}", min_signers, max_signers);
    println!(
        "  Need {} nodes to sign, can tolerate {} failures\n",
        min_signers,
        max_signers - min_signers
    );

    println!("═══════════════════════════════════════════════════════════");
    println!("  Step 1: Initialize SoftHSM Token for Each Node");
    println!("═══════════════════════════════════════════════════════════\n");

    for i in 0..max_signers {
        println!("Node {}:", i);
        println!("  ./scripts/init-softhsm.sh node{}", i);
        println!();
    }

    println!("This creates:");
    for i in 0..max_signers {
        println!("  - SoftHSM token: frost-node-{}", i);
        println!("  - Master key:    frost-master-key-node{}", i);
    }
    println!();

    println!("═══════════════════════════════════════════════════════════");
    println!("  Step 2: Configure Each Node");
    println!("═══════════════════════════════════════════════════════════\n");

    for i in 0..max_signers {
        println!("Node {} config (config-node{}.toml):", i, i);
        println!("---");
        println!("[server]");
        println!("role = \"node\"");
        println!("host = \"0.0.0.0\"");
        println!("port = 4000\n");
        println!("[node]");
        println!("index = {}", i);
        println!("storage_path = \"./data/frost-node{}\"", i);
        println!("max_signers = {}", max_signers);
        println!("min_signers = {}\n", min_signers);
        println!("[node.key_provider]");
        println!("pkcs11_library = \"/usr/lib/softhsm/libsofthsm2.so\"");
        println!("slot = 0");
        println!("pin = \"123456\"  # Use ${{HSM_PIN}} in production!");
        println!("key_label = \"frost-master-key-node{}\"", i);
        println!("---\n");
    }

    println!("═══════════════════════════════════════════════════════════");
    println!("  Step 3: Start Nodes");
    println!("═══════════════════════════════════════════════════════════\n");

    for i in 0..max_signers {
        println!(
            "Node {}: cargo run --bin frost-service -- config-node{}.toml",
            i, i
        );
    }
    println!();

    println!("═══════════════════════════════════════════════════════════");
    println!("  Production HSM Migration");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("To upgrade from SoftHSM to production HSM:");
    println!("  1. Generate key on production HSM:");
    println!("     pkcs11-tool --module /opt/cloudhsm/lib/libcloudhsm_pkcs11.so \\");
    println!("       --login --pin $PROD_PIN \\");
    println!("       --keypairgen --key-type EC:prime256v1 \\");
    println!("       --label frost-master-key-node0\n");
    println!("  2. Update config (only library path changes!):");
    println!("     pkcs11_library = \"/opt/cloudhsm/lib/libcloudhsm_pkcs11.so\"\n");
    println!("  3. Restart - same code, different hardware!\n");

    println!("Supported HSM devices:");
    println!("  - SoftHSM       (development, free)");
    println!("  - YubiKey 5     (small prod, $50)");
    println!("  - AWS CloudHSM  (cloud, $1K/month)");
    println!("  - Thales Luna   (enterprise, $5K+)\n");

    println!("Usage:");
    println!("  cargo run --bin frost-keygen           # 2-of-3 (default)");
    println!("  cargo run --bin frost-keygen 3 5       # 3-of-5");
    println!("  cargo run --bin frost-keygen 14 21     # 14-of-21");

    Ok(())
}
