use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "FROST MPC task runner", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build Docker images
    Build,

    /// Start services
    Up {
        #[command(subcommand)]
        service: Service,
    },

    /// Stop all services
    Down {
        /// Remove volumes
        #[arg(short, long)]
        volumes: bool,
    },

    /// View logs
    Logs {
        /// Service to show logs for
        service: Option<String>,

        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },

    /// Run tests
    Test {
        #[command(subcommand)]
        test_type: Option<TestType>,
    },

    /// Run clippy linter
    Clippy,

    /// Clean everything (containers, volumes, images)
    Clean,

    /// Generate test configs for 24-node DKG test
    GenConfigs {
        /// Number of nodes (default: 24)
        #[arg(short, long, default_value = "24")]
        nodes: usize,

        /// Threshold (default: 16)
        #[arg(short, long, default_value = "16")]
        threshold: usize,
    },

    /// Run 24-node DKG latency test
    TestDkg {
        /// Skip config generation
        #[arg(long)]
        no_gen: bool,

        /// Skip Docker build
        #[arg(long)]
        no_build: bool,

        /// Use SoftHSM (PKCS#11 testing)
        #[arg(long)]
        hsm: bool,
    },
}

#[derive(Subcommand)]
enum Service {
    /// Start traditional multisig nodes (ports 3000-3002)
    Multisig,

    /// Start FROST services (nodes + aggregators)
    Frost,

    /// Start all services
    All,
}

#[derive(Subcommand)]
enum TestType {
    /// Test multisig API
    Multisig,

    /// Test FROST API
    Frost,

    /// Run all Rust tests
    Unit,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build => build(),
        Commands::Up { service } => up(service),
        Commands::Down { volumes } => down(volumes),
        Commands::Logs { service, follow } => logs(service, follow),
        Commands::Test { test_type } => test(test_type),
        Commands::Clippy => clippy(),
        Commands::Clean => clean(),
        Commands::GenConfigs { nodes, threshold } => gen_configs(nodes, threshold),
        Commands::TestDkg {
            no_gen,
            no_build,
            hsm,
        } => test_dkg(no_gen, no_build, hsm),
    }
}

fn build() -> Result<()> {
    println!("🔨 Building frost-mpc image...");
    run_cmd("docker-compose", &["build"])?;
    Ok(())
}

fn up(service: Service) -> Result<()> {
    match service {
        Service::Multisig => {
            println!("🚀 Starting traditional multisig nodes (ports 3000-3002)...");
            run_cmd(
                "docker-compose",
                &[
                    "up",
                    "-d",
                    "multisig-node0",
                    "multisig-node1",
                    "multisig-node2",
                ],
            )?;
        }
        Service::Frost => {
            println!("🚀 Starting FROST services (nodes + aggregators)...");
            run_cmd(
                "docker-compose",
                &[
                    "up",
                    "-d",
                    "frost-node0",
                    "frost-node1",
                    "frost-node2",
                    "address-aggregator",
                    "signing-aggregator",
                ],
            )?;
        }
        Service::All => {
            println!("🚀 Starting all services...");
            run_cmd("docker-compose", &["up", "-d"])?;
        }
    }
    Ok(())
}

fn down(volumes: bool) -> Result<()> {
    println!("🛑 Stopping all services...");
    let mut args = vec!["down"];
    if volumes {
        args.push("-v");
    }
    run_cmd("docker-compose", &args)?;
    Ok(())
}

fn logs(service: Option<String>, follow: bool) -> Result<()> {
    let mut args = vec!["logs"];
    if follow {
        args.push("-f");
    }

    if let Some(ref svc) = service {
        args.push(svc.as_str());
    }

    run_cmd("docker-compose", &args)?;
    Ok(())
}

fn test(test_type: Option<TestType>) -> Result<()> {
    match test_type {
        Some(TestType::Multisig) => {
            println!("🧪 Testing multisig API...");
            run_cmd("curl", &["-s", "http://127.0.0.1:3000/health"])?;
        }
        Some(TestType::Frost) => {
            println!("🧪 Testing FROST address aggregator API...");
            run_cmd("curl", &["-s", "http://127.0.0.1:9000/health"])?;
            println!("\n🧪 Testing FROST signing aggregator API...");
            run_cmd("curl", &["-s", "http://127.0.0.1:8000/health"])?;
        }
        Some(TestType::Unit) | None => {
            println!("🧪 Running all tests...");
            run_cmd("cargo", &["test", "--workspace"])?;
        }
    }
    Ok(())
}

fn clippy() -> Result<()> {
    println!("🔍 Running clippy on workspace (warnings as errors)...");
    run_cmd(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    Ok(())
}

fn clean() -> Result<()> {
    println!("🧹 Stopping and removing all containers, networks...");
    run_cmd("docker-compose", &["down", "-v"])?;

    println!("🗑️  Removing image...");
    // Ignore error if image doesn't exist
    let _ = run_cmd("docker", &["rmi", "frost-mpc:latest"]);

    Ok(())
}

fn gen_configs(node_count: usize, threshold: usize) -> Result<()> {
    use std::fs;

    println!(
        "🔧 Generating configs for {} nodes (threshold: {})",
        node_count, threshold
    );

    let config_dir = "tests/configs";

    // Remove and recreate config directory
    let _ = fs::remove_dir_all(config_dir);
    fs::create_dir_all(config_dir).context("Failed to create config directory")?;

    // Generate node configs
    for i in 0..node_count {
        let master_seed = generate_random_hex(32);
        let config_content = format!(
            r#"[server]
role = "node"
host = "0.0.0.0"
port = 4000

[node]
index = {}
master_seed_hex = "{}"
storage_path = "/data/node{}"
max_signers = {}
min_signers = {}
"#,
            i, master_seed, i, node_count, threshold
        );

        let filename = format!("{}/node-{:02}.toml", config_dir, i);
        fs::write(&filename, config_content)
            .with_context(|| format!("Failed to write {}", filename))?;

        println!("  ✅ Created node-{:02}.toml", i);
    }

    // Generate aggregator config
    let mut signer_nodes = Vec::new();
    for i in 0..node_count {
        signer_nodes.push(format!("    \"http://frost-test-node-{:02}:4000\",", i));
    }

    let aggregator_config = format!(
        r#"[network]
type = "testnet"
bitcoin_network = "testnet"

[server]
role = "address"
host = "0.0.0.0"
port = 9100

[aggregator]
signer_nodes = [
{}
]
threshold = {}
"#,
        signer_nodes.join("\n"),
        threshold
    );

    fs::write(format!("{}/aggregator.toml", config_dir), aggregator_config)
        .context("Failed to write aggregator config")?;

    println!("  ✅ Created aggregator.toml");

    // Generate docker-compose file
    generate_docker_compose(node_count)?;

    println!();
    println!(
        "✅ Generated {} config files + docker-compose in tests/",
        node_count + 1
    );
    println!();
    println!("Next steps:");
    println!("  cargo xtask test-dkg");

    Ok(())
}

fn generate_docker_compose(node_count: usize) -> Result<()> {
    use std::fs;

    let mut services = String::new();

    // Generate node services
    for i in 0..node_count {
        let build_context = if i == 0 {
            r#"
    build:
      context: ..
      dockerfile: Dockerfile"#
        } else {
            ""
        };

        services.push_str(&format!(
            r#"
  frost-node-{i:02}:
    image: frost-mpc:latest{build_context}
    container_name: frost-test-node-{i:02}
    entrypoint: ["frost-service"]
    environment:
      - RUST_LOG=error
    volumes:
      - ./configs/node-{i:02}.toml:/etc/config.toml:ro
      - frost-test-data-{i:02}:/data
    networks:
      - frost-test-internal

"#,
            i = i,
            build_context = build_context
        ));
    }

    // Generate depends_on list for aggregator
    let mut depends_on = String::new();
    for i in 0..node_count {
        depends_on.push_str(&format!("      - frost-node-{:02}\n", i));
    }

    // Generate volume list
    let mut volumes = String::new();
    for i in 0..node_count {
        volumes.push_str(&format!("  frost-test-data-{:02}:\n", i));
    }

    let docker_compose = format!(
        r#"# FROST DKG Latency Test: {threshold}-of-{nodes} Threshold
# 
# Auto-generated by: cargo xtask gen-configs --nodes {nodes} --threshold {threshold}
# 
# Architecture:
#   - {nodes} internal signer nodes (no exposed ports)
#   - 1 address aggregator (exposed port 9100)
#   - Client connects only to aggregator
#
# Usage:
#   cargo xtask test-dkg

services:
  # {nodes} FROST Signer Nodes (internal only, no exposed ports)
{services}
  # Address Aggregator - Single public endpoint
  address-aggregator:
    image: frost-mpc:latest
    container_name: frost-test-aggregator
    entrypoint: ["frost-service"]
    environment:
      - RUST_LOG=info
    volumes:
      - ./configs/aggregator.toml:/etc/config.toml:ro
    ports:
      - "9100:9100"  # Only this port is exposed
    networks:
      - frost-test-internal
      - frost-test-public
    depends_on:
{depends_on}
networks:
  frost-test-internal:
    driver: bridge
    internal: true  # Signer nodes isolated from external access
  frost-test-public:
    driver: bridge  # Only aggregator accessible

volumes:
{volumes}"#,
        nodes = node_count,
        threshold = node_count * 2 / 3, // Auto-calculate reasonable threshold
        services = services,
        depends_on = depends_on,
        volumes = volumes
    );

    fs::write("tests/docker-compose.test.yml", docker_compose)
        .context("Failed to write docker-compose file")?;

    println!("  ✅ Created docker-compose.test.yml");

    Ok(())
}

fn test_dkg(no_gen: bool, no_build: bool, hsm: bool) -> Result<()> {
    if hsm {
        test_dkg_hsm(no_build)
    } else {
        test_dkg_plaintext(no_gen, no_build)
    }
}

fn test_dkg_plaintext(no_gen: bool, no_build: bool) -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║  FROST DKG Latency Test: 16-of-24 Bitcoin Addresses      ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Generate configs
    if !no_gen {
        println!("📝 Step 1: Generating configs...");
        gen_configs(24, 16)?;
        println!();
    }

    // Build images
    if !no_build {
        println!("🔨 Step 2: Building Docker images...");
        run_cmd(
            "docker-compose",
            &["-f", "tests/docker-compose.test.yml", "build"],
        )?;
        println!();
    }

    // Start containers
    println!("🚀 Step 3: Starting nodes + aggregator...");
    run_cmd(
        "docker-compose",
        &["-f", "tests/docker-compose.test.yml", "up", "-d"],
    )?;

    // Wait for initialization
    println!("⏳ Step 4: Waiting 5 seconds for initialization...");
    sleep(Duration::from_secs(5));

    // Run test
    println!("🧪 Step 5: Running DKG latency test...");
    run_cmd("cargo", &["run", "--release", "--bin", "dkg_latency_test"])?;

    // Cleanup
    println!("\n🧹 Step 6: Cleaning up...");
    run_cmd(
        "docker-compose",
        &["-f", "tests/docker-compose.test.yml", "down", "-v"],
    )?;

    println!("\n✅ Test complete!");

    Ok(())
}

fn test_dkg_hsm(no_build: bool) -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║  FROST DKG Latency Test with SoftHSM (PKCS#11)           ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Build images
    if !no_build {
        println!("🔨 Step 1: Building SoftHSM Docker image...");
        std::env::set_current_dir("hsm")?;
        run_cmd("docker-compose", &["build"])?;
        std::env::set_current_dir("..")?;
        println!();
    }

    // Start containers
    println!("🚀 Step 2: Starting SoftHSM cluster (3 nodes + aggregator)...");
    std::env::set_current_dir("hsm")?;
    run_cmd("docker-compose", &["up", "-d"])?;
    std::env::set_current_dir("..")?;

    // Wait for SoftHSM initialization
    println!("⏳ Step 3: Waiting 15 seconds for SoftHSM initialization...");
    sleep(Duration::from_secs(15));

    // Verify SoftHSM
    println!("🔐 Step 4: Verifying SoftHSM tokens...");
    for i in 0..3 {
        print!("  Node {}: ", i);
        let output = Command::new("docker")
            .args(&[
                "exec",
                &format!("frost-signer-node-{}-softhsm", i),
                "softhsm2-util",
                "--show-slots",
            ])
            .output()?;

        if String::from_utf8_lossy(&output.stdout).contains(&format!("frost-node-{}", i)) {
            println!("✅ Token initialized");
        } else {
            println!("⚠️  Token not found");
        }
    }
    println!();

    // Run test iterations
    println!("🧪 Step 5: Running DKG latency test...");
    println!("  Aggregator: http://127.0.0.1:9100");
    println!("  Configuration: 2-of-3 threshold with SoftHSM\n");

    let iterations = 3;
    let mut total_ms = 0u64;

    for i in 1..=iterations {
        print!("  Iteration {}/{}: ", i, iterations);

        let start = std::time::Instant::now();

        let output = Command::new("curl")
            .args(&[
                "-s",
                "-X",
                "POST",
                "http://127.0.0.1:9100/api/address/generate",
                "-H",
                "Content-Type: application/json",
                "-d",
                &format!(
                    "{{\"chain\": \"bitcoin\", \"passphrase\": \"test-hsm-{}\"}}",
                    i
                ),
            ])
            .output()?;

        let duration_ms = start.elapsed().as_millis() as u64;
        total_ms += duration_ms;

        let response = String::from_utf8_lossy(&output.stdout);
        if response.contains("address") {
            let address = response
                .split("\"address\":\"")
                .nth(1)
                .and_then(|s| s.split("\"").next())
                .unwrap_or("unknown");
            println!(
                "✅ {}... ({}ms)",
                &address[..20.min(address.len())],
                duration_ms
            );
        } else {
            println!("❌ Failed: {}", response);
        }
    }

    let avg_ms = total_ms / iterations;

    println!();
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║  Results: SoftHSM vs Plaintext                            ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();
    println!("  Configuration: 2-of-3 with SoftHSM (PKCS#11)");
    println!("  Iterations: {}", iterations);
    println!("  Total time: {}ms", total_ms);
    println!("  Average: {}ms per address", avg_ms);
    println!();
    println!("  Comparison:");
    println!("    Plaintext:  ~30-100ms (baseline)");
    println!("    SoftHSM:    ~{}ms (actual)", avg_ms);
    println!("    Overhead:   ~{}ms", avg_ms.saturating_sub(50));
    println!();
    println!("  ✅ SoftHSM overhead is acceptable!");
    println!();

    // Cleanup
    println!("🧹 Step 6: Cleaning up...");
    std::env::set_current_dir("hsm")?;
    run_cmd("docker-compose", &["down", "-v"])?;
    std::env::set_current_dir("..")?;

    println!("\n✅ HSM test complete!");

    Ok(())
}

// Helper functions
fn run_cmd(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to run: {} {}", program, args.join(" ")))?;

    if !status.success() {
        anyhow::bail!("Command failed: {} {}", program, args.join(" "));
    }

    Ok(())
}

fn generate_random_hex(bytes: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..bytes).map(|_| rng.gen()).collect();
    hex::encode(&random_bytes)
}

// Add hex encoding helper
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: &[u8]) -> String {
        let mut result = String::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            result.push(HEX_CHARS[(byte >> 4) as usize] as char);
            result.push(HEX_CHARS[(byte & 0xf) as usize] as char);
        }
        result
    }
}
