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
        Commands::TestDkg { no_gen, no_build } => test_dkg(no_gen, no_build),
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
        "🔧 Generating configs for {} nodes (threshold: {}) with SoftHSM",
        node_count, threshold
    );

    let config_dir = "tests/configs";

    // Remove and recreate config directory
    let _ = fs::remove_dir_all(config_dir);
    fs::create_dir_all(config_dir).context("Failed to create config directory")?;

    // Generate node configs with PKCS#11 configuration
    for i in 0..node_count {
        let config_content = format!(
            r#"[server]
role = "node"
host = "0.0.0.0"
port = 4000

[node]
index = {}
storage_path = "/data/node{}"
max_signers = {}
min_signers = {}

[node.key_provider]
type = "pkcs11"
pkcs11_library = "/usr/lib/softhsm/libsofthsm2.so"
slot = 0
pin = "123456"
key_label = "frost-master-key-node{}"
"#,
            i, i, node_count, threshold, i
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
    command: ["frost-service"]
    environment:
      - RUST_LOG=error
      - NODE_INDEX={i}
    volumes:
      - ./configs/node-{i:02}.toml:/etc/config.toml:ro
      - frost-test-data-{i:02}:/data
      - frost-test-softhsm-{i:02}:/var/lib/softhsm/tokens  # Cache HSM keys between runs
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

    // Generate volume list (data + SoftHSM tokens)
    let mut volumes = String::new();
    for i in 0..node_count {
        volumes.push_str(&format!("  frost-test-data-{:02}:\n", i));
        volumes.push_str(&format!(
            "  frost-test-softhsm-{:02}:  # Cached HSM keys\n",
            i
        ));
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
# Performance:
#   First run:  ~900ms (generates SoftHSM keys)
#   Subsequent: ~900ms (reuses cached keys from volumes)
#   
#   To reset keys: docker-compose -f tests/docker-compose.test.yml down -v
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

fn test_dkg(no_gen: bool, no_build: bool) -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║  FROST DKG Latency Test: 16-of-24 with SoftHSM           ║");
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

    println!("\n✅ Test complete with SoftHSM!");
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
