# xtask - FROST MPC Task Runner

Rust-based task runner that replaces Makefiles and shell scripts with a single, type-safe binary.

## Installation

The xtask is automatically available when you build the workspace:

```bash
cargo build
```

Then use the `cargo xtask` alias to run tasks:

```bash
cargo xtask --help
```

## Available Commands

### Production Services

```bash
# Build Docker images
cargo xtask build

# Start multisig nodes (ports 3000-3002)
cargo xtask up multisig

# Start FROST services (nodes + aggregators)
cargo xtask up frost

# Start all services
cargo xtask up all

# Stop services
cargo xtask down

# Stop and remove volumes
cargo xtask down --volumes

# View logs
cargo xtask logs
cargo xtask logs --follow
cargo xtask logs <service-name>
```

### Testing

```bash
# Test multisig API
cargo xtask test multisig

# Test FROST API
cargo xtask test frost

# Run Rust unit tests
cargo xtask test unit
cargo xtask test  # same as unit

# Run clippy linter
cargo xtask clippy
```

### DKG Latency Test (16-of-24)

```bash
# Generate configs for 24 nodes
cargo xtask gen-configs

# Custom node count and threshold
cargo xtask gen-configs --nodes 15 --threshold 10

# Run complete DKG latency test
cargo xtask test-dkg

# Skip config generation
cargo xtask test-dkg --no-gen

# Skip Docker build
cargo xtask test-dkg --no-build
```

### Cleanup

```bash
# Stop all containers
cargo xtask down

# Stop and remove volumes
cargo xtask down --volumes

# Complete cleanup (containers + volumes + images)
cargo xtask clean
```

## Examples

### Quick Start for Development

```bash
# Build and start FROST services
cargo xtask build
cargo xtask up frost

# View logs
cargo xtask logs --follow

# Stop when done
cargo xtask down --volumes
```

### Running the 24-Node DKG Test

```bash
# Complete automated test
cargo xtask test-dkg
```

This will:
1. Generate 24 node configs + aggregator config
2. Build Docker images
3. Start all 24 nodes + aggregator
4. Wait 60 seconds for initialization
5. Run the DKG latency test
6. Clean up containers

### Development Workflow

```bash
# Run clippy before commits
cargo xtask clippy

# Run tests
cargo xtask test

# Start services for manual testing
cargo xtask up frost
cargo xtask logs --follow
```

## Why xtask Instead of Make?

### Advantages

1. **Type Safety**: Rust compiler catches errors at compile time
2. **Cross-Platform**: Works on Windows, Mac, Linux without shell differences
3. **No External Dependencies**: Just Rust (no make, bash, etc.)
4. **Better Error Handling**: Rust's Result type vs shell exit codes
5. **Code Reuse**: Can share code between tasks
6. **IDE Support**: Full autocomplete and navigation

### Comparison

**Before (Makefile)**:
```makefile
test-dkg:
    ./tests/generate_configs.sh
    docker-compose -f tests/docker-compose.test-24.yml up -d
    sleep 60
    cargo run --release --bin dkg_latency_test
```

**After (xtask)**:
```bash
cargo xtask test-dkg
```

Much cleaner and works everywhere!

## Implementation Details

The xtask is a regular Rust binary that:
- Uses `clap` for CLI parsing
- Calls external commands via `std::process::Command`
- Generates configs programmatically (no shell scripts)
- Provides nice progress output with emojis

See `xtask/src/main.rs` for the implementation.

## Adding New Tasks

To add a new task:

1. Add a new variant to the `Commands` enum:
```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    
    /// Your new task
    MyTask {
        #[arg(short, long)]
        option: String,
    },
}
```

2. Handle it in `main()`:
```rust
match cli.command {
    // ... existing matches
    Commands::MyTask { option } => my_task(option),
}
```

3. Implement the function:
```rust
fn my_task(option: String) -> Result<()> {
    println!("Running my task with option: {}", option);
    run_cmd("some-command", &["arg1", &option])?;
    Ok(())
}
```

That's it! The task is now available via `cargo xtask my-task`.

