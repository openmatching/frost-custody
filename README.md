# FROST MPC

**Production-ready Multi-Party Computation threshold signature infrastructure**

Supporting Bitcoin, Ethereum, and Solana with flexible **m-of-n** threshold signatures using FROST protocol.

## Quick Start

```bash
# Build and start FROST services
cargo xtask build
cargo xtask up frost

# Generate addresses
curl -X POST http://localhost:9000/api/address/generate \
  -H "Content-Type: application/json" \
  -d '{"chain": "bitcoin", "passphrase": "user-wallet-001"}'

# Returns: {"address": "bc1p...", "public_key": "03...", "curve": "secp256k1-tr"}

# View logs
cargo xtask logs --follow

# Stop services
cargo xtask down
```

## Running Examples

### Bitcoin Address Generation

```bash
# Start services
cargo xtask up frost

# Generate Bitcoin address via client library
cargo run --example derive_address

# Or use FROST threshold address generation
curl -X POST http://localhost:9000/api/address/generate \
  -H "Content-Type: application/json" \
  -d '{"chain": "bitcoin", "passphrase": "unique-wallet-id"}'
```

### Bitcoin Transaction Signing (FROST)

```bash
# Example: Sign Bitcoin PSBT with 2-of-3 threshold
cargo run --example sign_psbt_frost

# This will:
# 1. Generate 3 addresses via FROST DKG
# 2. Create a Bitcoin transaction
# 3. Sign with 2-of-3 threshold signature
```

### Multi-Chain Examples

```bash
# Ethereum signing
cargo run --example sign_eth_frost

# Solana signing  
cargo run --example sign_sol_frost
```

## Testing

### Run All Tests

```bash
cargo xtask test      # Unit tests
cargo xtask clippy    # Linter
```

### DKG Latency Test (16-of-24 Nodes)

Comprehensive test with 24 signer nodes and 16-of-24 threshold:

```bash
# Complete automated test
cargo xtask test-dkg
```

**Measured Performance (16-of-24 threshold, local Docker):**
- Average latency: **32ms** per address
- Byzantine tolerance: 8 compromised nodes
- Test consistency: 26-41ms range across 3 runs

**Note:** Each DKG is independent - you can run multiple concurrent requests for much higher throughput (10 concurrent = 300+ addr/sec).

This command will:
1. Generate 24 node configs + aggregator config + docker-compose file
2. Build Docker images
3. Start all 24 nodes + aggregator
4. Run DKG latency measurements (3 iterations)
5. Clean up everything

**Custom configurations:**

```bash
# Test with 10 nodes, 7-of-10 threshold
cargo xtask gen-configs --nodes 10 --threshold 7
cargo xtask test-dkg

# Test with 15 nodes, 10-of-15 threshold
cargo xtask gen-configs --nodes 15 --threshold 10
cargo xtask test-dkg
```

The test generates all necessary files dynamically based on your parameters.

**Performance expectations:**
- Local Docker: 30-150ms (measured)
- Same datacenter: 100-400ms (estimated)
- Multi-datacenter: 500-2000ms (estimated)

---

## API Reference

### Generate Addresses

**Bitcoin:**
```bash
curl -X POST http://localhost:9000/api/address/generate \
  -H "Content-Type: application/json" \
  -d '{"chain": "bitcoin", "passphrase": "unique-id-123"}'
```

**Ethereum:**
```bash
curl -X POST http://localhost:9000/api/address/generate \
  -H "Content-Type: application/json" \
  -d '{"chain": "ethereum", "passphrase": "unique-id-456"}'
```

**Solana:**
```bash
curl -X POST http://localhost:9000/api/address/generate \
  -H "Content-Type: application/json" \
  -d '{"chain": "solana", "passphrase": "unique-id-789"}'
```

### Sign Messages/Transactions

```bash
curl -X POST http://localhost:8000/api/sign/message \
  -H "Content-Type: application/json" \
  -d '{
    "passphrase": "unique-id-123",
    "message": "SGVsbG8gV29ybGQ="
  }'
```

See API documentation: http://localhost:9000/docs

---

## Troubleshooting

```bash
# Check service status
docker ps

# View logs
cargo xtask logs --follow
cargo xtask logs address-aggregator
cargo xtask logs signing-aggregator

# Rebuild everything
cargo xtask clean
cargo xtask build
cargo xtask up frost

# Verify services are responding
curl http://localhost:9000/docs  # Address aggregator
curl http://localhost:8000/docs  # Signing aggregator
```

---

## Command Reference

| Task                  | Command                       |
| --------------------- | ----------------------------- |
| See all commands      | `cargo xtask --help`          |
| Build images          | `cargo xtask build`           |
| Start FROST           | `cargo xtask up frost`        |
| Start FROST (SoftHSM) | `cd hsm && docker-compose up` |
| Start all services    | `cargo xtask up all`          |
| View logs             | `cargo xtask logs --follow`   |
| Run tests             | `cargo xtask test`            |
| Run clippy            | `cargo xtask clippy`          |
| Generate test configs | `cargo xtask gen-configs`     |
| DKG latency test      | `cargo xtask test-dkg`        |
| Stop services         | `cargo xtask down`            |
| Complete cleanup      | `cargo xtask clean`           |

---

## Architecture

### Three-Curve Design

| Curve        | Signature Type | Blockchain   | Use Case          |
| ------------ | -------------- | ------------ | ----------------- |
| secp256k1-tr | Schnorr        | Bitcoin      | Taproot key-spend |
| secp256k1    | ECDSA          | Ethereum/EVM | Standard txs      |
| ed25519      | Ed25519        | Solana       | Native txs        |

### Components

```
Client
  ├─→ Address Aggregator (port 9000) - LOW RISK
  │   └─ Generates addresses, runs DKG automatically
  │
  └─→ Signing Aggregator (port 8000) - HIGH RISK  
      └─ Signs transactions with threshold signatures
            ↓
      Signer Nodes (internal network)
      └─ Pure crypto operations (curve-agnostic)
```

**See [ARCHITECTURE.md](ARCHITECTURE.md) for details.**

---

## Examples

### Bitcoin PSBT Signing

```bash
cargo run --example sign_psbt_frost
```

- Generate Taproot addresses
- Build PSBT with multiple inputs
- Sign with FROST threshold signatures (configurable m-of-n)
- **~110 vB (56% smaller than multisig!)**

### Ethereum Transaction Signing

```bash
cargo run --example sign_eth_frost
```

- Generate Ethereum address (ECDSA FROST)
- Build EIP-155 transaction
- Sign with threshold signatures
- Uses `ethers-core` for real RLP encoding
- **Server-side validation recommended**

### Solana Transaction Signing

```bash
cargo run --example sign_sol_frost
```

- Generate Solana address (Ed25519 FROST)
- Build transaction with `solana-sdk`
- Sign with Ed25519 threshold signatures
- **Cryptographically verified with `ed25519-dalek`**

---

## API

### Generate Address

```bash
POST /api/address/generate
{
  "chain": "bitcoin|ethereum|solana",
  "passphrase": "unique-passphrase"
}

Response:
{
  "address": "...",
  "public_key": "...",  # For signature verification
  "curve": "secp256k1-tr|secp256k1|ed25519",
  "chain": "...",
  "passphrase": "..."
}
```

### Sign Message/Transaction

```bash
POST /api/sign/message
{
  "passphrase": "user-passphrase",
  "message": "hex-encoded-hash",
  "curve": "secp256k1-tr|secp256k1|ed25519"  # Optional, defaults to secp256k1-tr
}

Response:
{
  "signature": "hex-encoded-signature",
  "verified": true  # FROST verified!
}
```

### Sign Bitcoin PSBT

```bash
POST /api/sign/psbt
{
  "psbt": "base64-encoded-psbt",
  "passphrases": ["pass1", "pass2"]  # One per input
}

Response:
{
  "signed_psbt": "base64-encoded-signed-psbt",
  "signatures_added": 2
}
```

---

## Deployment

```bash
# Build
make build

# Start FROST services (3 nodes + 2 aggregators)
make up-frost

# Check health
curl http://localhost:9000/health  # Address aggregator
curl http://localhost:8000/health  # Signing aggregator

# View logs
make logs-frost
```

### Services

| Service              | Port | Role    | Risk     | Expose? |
| -------------------- | ---- | ------- | -------- | ------- |
| frost-node-0,1,2,... | 4000 | Signer  | HIGH     | NO      |
| address-aggregator   | 9000 | DKG     | LOW      | YES     |
| signing-aggregator   | 8000 | Signing | CRITICAL | NO      |

**Note:** Configure any number of signer nodes (m-of-n threshold)

---

## Security

### Threshold Security (m-of-n)

**Flexible threshold configuration:**
- 2-of-3 (demo/standard)
- 3-of-5 (recommended)
- 5-of-7 (high security)
- Any m-of-n combination

**Security properties:**
- m-1 nodes compromised → funds safe ✅
- m nodes compromised → funds at risk ⚠️

### Network Segmentation

**Production deployment:**
- Signer nodes: Internal network ONLY
- Address aggregator: Can expose (generates addresses, can't sign)
- Signing aggregator: VPN/internal ONLY (can sign transactions!)

### Key Management

**Backup:**
- Master seeds (one per signer node) - can be plaintext or hardware-backed
- List of passphrases (from your database)

**Recovery:**
- Re-run DKG for each passphrase
- Deterministic → same keys recovered

**Hardware Security:**
- PKCS#11 support (enabled by default)
- Works with YubiKey ($55), Thales HSM, AWS CloudHSM, or any PKCS#11 device
- Master key never in plaintext
- Test with: `cargo xtask test-dkg --hsm` (SoftHSM)
- Setup: [CONFIG_HSM.md](frost-service/CONFIG_HSM.md)

**See [SECURITY.md](SECURITY.md) and [SYSTEM_DESIGN.md](SYSTEM_DESIGN.md) for deployment guide.**

---

## Why FROST?

### Why MPC with FROST?

**MPC (Multi-Party Computation)** allows multiple parties to jointly compute signatures without ever reconstructing the private key.

**FROST (Flexible Round-Optimized Schnorr Threshold)** provides:
- ✅ Threshold signatures (m-of-n)
- ✅ No single point of failure
- ✅ Smaller transaction sizes than multisig

**Fee Savings (Bitcoin):**

| Implementation   | Tx Size     | Fee @ 50 sat/vB | Annual @ 1000 tx/day |
| ---------------- | ----------- | --------------- | -------------------- |
| Multisig (P2WSH) | ~250 vB     | 12,500 sats     | $2.3M                |
| **FROST (P2TR)** | **~110 vB** | **5,500 sats**  | **$1.0M**            |

**Savings: $1.3M per year with same security!** 💰

### Multi-Chain Support

**One secp256k1 key → Multiple chains?**

**No!** Bitcoin and Ethereum use **different keys**:
- Bitcoin uses `secp256k1-tr` (Schnorr/Taproot)
- Ethereum uses `secp256k1` (ECDSA)
- Different signature schemes → different keys (for security!)

**Same passphrase generates:**
- Bitcoin address from Schnorr key
- Ethereum address from ECDSA key  
- Solana address from Ed25519 key

All **independent and isolated**.

---

## Development

### Build

```bash
cargo build --release
make build
```

### Test

```bash
# Unit tests
cargo test --workspace

# Examples
cargo run --example sign_psbt_frost    # Bitcoin
cargo run --example sign_eth_frost     # Ethereum
cargo run --example sign_sol_frost     # Solana

# Health checks
make test-frost
```

### Configuration

Each service needs a config file:

```toml
[network]
type = "mainnet"  # or "testnet"

[server]
role = "node"           # node | address | signer
host = "0.0.0.0"
port = 4000

[node]
index = 0
master_seed_hex = "..."  # OR use hardware HSM (see below)
storage_path = "./data/node0"
max_signers = 5         # n (total number of signers)
min_signers = 3         # m (minimum required to sign)
```

**Threshold Configuration:**
- `max_signers` = n (total number of signer nodes)
- `min_signers` = m (minimum required for signing)
- Examples: 2-of-3, 3-of-5, 5-of-7, or any m-of-n

**Hardware Security (PKCS#11 - enabled by default):**

```toml
[node.key_provider]
type = "pkcs11"
pkcs11_library = "/usr/lib/libykcs11.so"  # YubiKey, Thales, AWS CloudHSM, etc.
slot = 0
# pin = "${HSM_PIN}"  # Optional: omit for unlock via API (more secure)
key_label = "frost-node-0"
```

**Unlock API (no PIN in config):**
```bash
# Start node (HSM locked)
cargo run --release

# Unlock via API
curl -X POST http://localhost:4000/api/hsm/unlock -d '{"pin": "123456"}'

# Check status
curl http://localhost:4000/api/hsm/status
```

Works with ANY PKCS#11 device. Test with SoftHSM: `cargo xtask test-dkg --hsm`

See `config-pkcs11.toml.example`, `config-pkcs11-nopin.toml.example`, and `frost-service/CONFIG_HSM.md`.

---

## Adding New Blockchains

### Add EVM Chain (Polygon, BSC, Arbitrum)

**Signer nodes:** ZERO changes  
**Signing aggregator:** ZERO changes  
**Address aggregator:** ~10 lines

```rust
// In address_aggregator/chain_derivation.rs
pub enum Chain {
    Bitcoin,
    Ethereum,
    Polygon,  // ← Add this
    Solana,
}

// Map to curve (Polygon uses ECDSA like Ethereum)
Chain::Ethereum | Chain::Polygon => ("secp256k1", "secp256k1"),

// Derive address (same as Ethereum!)
Chain::Ethereum | Chain::Polygon => derive_ethereum_address(&pubkey_hex)?,
```

### Add Ed25519 Chain (Cosmos, Cardano)

**Signer nodes:** ZERO changes  
**Signing aggregator:** ZERO changes  
**Address aggregator:** Add address encoding

```rust
Chain::Cosmos => {
    // Use ed25519 DKG (already implemented)
    derive_cosmos_address(&pubkey_hex)?  // Add Bech32 encoding
}
```

**This is the power of chain-agnostic signer design.**

---

## Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System architecture
- **[SECURITY.md](SECURITY.md)** - Security model and passphrase best practices
- **[SYSTEM_DESIGN.md](SYSTEM_DESIGN.md)** - Deployment, threat model, and HSM options
- **[frost-service/CONFIG_HSM.md](frost-service/CONFIG_HSM.md)** - PKCS#11 setup (YubiKey, Thales, AWS)

---

## Production Notes

### Ethereum Signature Verification

For custody applications, **validate signatures server-side**:

```typescript
// You have the public key from address generation
const { address, public_key } = await generateAddress('ethereum', passphrase);

// Later, verify signature
const valid = secp256k1.verify(messageHash, signature, publicKey);

if (valid && tx.from === address) {
    await ethClient.sendRawTransaction(signedTx);
}
```

**Don't use `ecrecover()`** for custody - you already know the signer!

`ecrecover()` is only needed for:
- Smart contracts verifying signatures on-chain
- Permissionless systems where signer is unknown

For these cases, consider server-side re-signing or hybrid approaches.

---

## FAQ

**Q: Do Bitcoin and Ethereum share the same FROST key?**

A: **No!** They use different curves:
- Bitcoin: `secp256k1-tr` (Schnorr/Taproot)
- Ethereum: `secp256k1` (ECDSA)
- Same passphrase → different keys (intentional for security)

**Q: Can I add Polygon, BSC, Avalanche?**

A: **Yes!** All EVM chains use ECDSA like Ethereum. Just add chain enum and change chain_id. Zero signer changes needed.

**Q: How do I recover keys?**

A: Keep 3 master seeds + list of passphrases. Re-run DKG for each passphrase (deterministic).

**Q: Is this production-ready?**

A: **Yes** for custody applications:
- ✅ Bitcoin: Complete PSBT signing
- ✅ Ethereum: ECDSA signing (server-side validation)
- ✅ Solana: Ed25519 signing (cryptographically verified)

**Q: Why separate Schnorr and ECDSA?**

A: Different signature algorithms have different security properties. Isolating them provides:
- Clear API semantics
- Better security (defense in depth)
- Explicit routing (no ambiguity)

---

## License

MIT License - See [LICENSE](LICENSE) for details
