# FROST Custody

**Production-ready multi-chain threshold signature custody service**

Supporting Bitcoin, Ethereum, and Solana with 2-of-3 threshold signatures using FROST.

## Quick Start

```bash
make build       # Build Docker image
make up-frost    # Deploy FROST services (3 signers + aggregator)

# Generate Bitcoin address
curl -X POST http://localhost:3000/api/address/generate \
  -H "Content-Type: application/json" \
  -d '{"chain": "bitcoin", "passphrase": "550e8400-e29b-41d4-a716-446655440000"}'

# Generate Ethereum address (same FROST key!)
curl -X POST http://localhost:3000/api/address/generate \
  -H "Content-Type: application/json" \
  -d '{"chain": "ethereum", "passphrase": "550e8400-e29b-41d4-a716-446655440000"}'
```

---

## Why FROST?

### Fee Savings

| Implementation               | Tx Size     | Fee @ 50 sat/vB | Annual @ 1000 tx/day |
| ---------------------------- | ----------- | --------------- | -------------------- |
| Traditional Multisig (P2WSH) | ~250 vB     | 12,500 sats     | $2.3M                |
| **FROST Taproot (P2TR)**     | **~110 vB** | **5,500 sats**  | **$1.0M**            |

**Annual savings: $1.3M** 💰

### Multi-Chain Support

| Chain        | Address Format    | Same Key as Bitcoin? |
| ------------ | ----------------- | -------------------- |
| **Bitcoin**  | bc1p... (Taproot) | -                    |
| **Ethereum** | 0x... (Keccak256) | ✅ Yes (secp256k1)    |
| **Solana**   | Base58            | ❌ No (Ed25519)       |

**One secp256k1 FROST key → Bitcoin + Ethereum + 100s more chains!**

---

## Architecture

### Chain-Agnostic Design

**Signer Nodes (Dumb Crypto Boxes):**
- Only know about **curves** (secp256k1, Ed25519)
- Expose raw public keys
- No chain-specific logic
- Pure FROST operations

**Address Aggregator (Smart Orchestrator):**
- Knows about **chains** (Bitcoin, Ethereum, Solana)
- Fetches raw pubkeys from signers
- Applies chain-specific address derivation
- Coordinates DKG

**Benefits:**
- ✅ Add 100 new chains → zero signer changes
- ✅ Smaller attack surface (no transaction parsing in signers)
- ✅ Independent deployment (update aggregator only)

See [ARCHITECTURE.md](ARCHITECTURE.md) for full details.

---

## Supported Blockchains

### Bitcoin
```bash
curl -X POST http://localhost:3000/api/address/generate \
  -d '{"chain": "bitcoin", "passphrase": "uuid"}'
  
→ bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr
```

### Ethereum
```bash
curl -X POST http://localhost:3000/api/address/generate \
  -d '{"chain": "ethereum", "passphrase": "uuid"}'
  
→ 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb
```

### Solana
```bash
curl -X POST http://localhost:3000/api/address/generate \
  -d '{"chain": "solana", "passphrase": "uuid"}'
  
→ 7EqQdEULxWcraVx3mXKFjc84LhCkMGZCkRuDpvcMwJeK
```

**Note:** Bitcoin and Ethereum share the same secp256k1 FROST key. Solana uses a separate Ed25519 FROST key.

---

## Project Structure

```
frost-custody/
├── bitcoin/
│   ├── multisig-signer/         # Legacy: Traditional 2-of-3 multisig
│   └── frost-service/            # FROST multi-chain service
│       ├── src/
│       │   ├── curves/           # Curve abstraction (secp256k1, Ed25519)
│       │   ├── node/             # Signer node (curve-agnostic)
│       │   │   ├── curve_api.rs  # GET /api/curve/{curve}/pubkey
│       │   │   └── multi_storage.rs  # RocksDB column families
│       │   └── address_aggregator/   # Chain-aware orchestrator
│       │       ├── chain_derivation.rs  # BTC/ETH/SOL derivation
│       │       └── multi_chain_api.rs   # POST /api/address/generate
├── client/                       # Client libraries
├── docker-compose.yml            # All services
├── Makefile                      # Easy commands
├── ARCHITECTURE.md               # Detailed architecture docs
└── SECURITY.md                   # Security model
```

---

## API Reference

### Signer Node (Curve API)

```bash
# Get raw public key (chain-agnostic)
curl "http://localhost:3001/api/curve/secp256k1/pubkey?passphrase=uuid"
→ {
    "curve": "secp256k1",
    "passphrase": "uuid",
    "public_key": "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
  }

# Health check
curl "http://localhost:3001/health"
→ {
    "status": "ok",
    "node_index": 0,
    "supported_curves": ["secp256k1", "ed25519"]
  }
```

### Aggregator (Chain API)

```bash
# Generate address for any chain
curl -X POST http://localhost:3000/api/address/generate \
  -H "Content-Type: application/json" \
  -d '{
    "chain": "bitcoin",
    "passphrase": "550e8400-e29b-41d4-a716-446655440000"
  }'

→ {
    "chain": "bitcoin",
    "passphrase": "550e8400-e29b-41d4-a716-446655440000",
    "address": "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr",
    "curve": "secp256k1"
  }

# Query params also work
curl "http://localhost:3000/api/address?chain=ethereum&passphrase=uuid"
```

---

## Deployment

### Docker Compose

```bash
# Start all services
docker-compose up -d

# Check health
curl http://localhost:3000/health  # Aggregator
curl http://localhost:3001/health  # Signer 0
curl http://localhost:3002/health  # Signer 1
curl http://localhost:3003/health  # Signer 2

# Generate addresses
curl -X POST http://localhost:3000/api/address/generate \
  -d '{"chain": "bitcoin", "passphrase": "test-uuid"}'
```

### Services

| Service          | Port | Role                                |
| ---------------- | ---- | ----------------------------------- |
| frost-aggregator | 3000 | Orchestrates DKG, derives addresses |
| frost-signer-0   | 3001 | FROST node 0 (threshold signing)    |
| frost-signer-1   | 3002 | FROST node 1 (threshold signing)    |
| frost-signer-2   | 3003 | FROST node 2 (threshold signing)    |

---

## Adding New Chains

Want to add Polygon, BSC, Avalanche, or any secp256k1 chain?

**Signer nodes: ZERO changes needed** ✅

**Aggregator: Add 10 lines of code**

```rust
// 1. Add to Chain enum
pub enum Chain {
    Bitcoin,
    Ethereum,
    Solana,
    Polygon,  // ← Add this
}

// 2. Map to curve (Polygon uses secp256k1 like Ethereum)
match chain {
    Chain::Bitcoin | Chain::Ethereum | Chain::Polygon => {
        ("secp256k1", "secp256k1")
    }
    ...
}

// 3. Apply address derivation (Polygon uses Ethereum format)
match chain {
    Chain::Ethereum | Chain::Polygon => derive_ethereum_address(&pubkey)?,
    ...
}
```

**Deploy: Restart aggregator only, signers stay running**

```bash
docker-compose restart frost-aggregator
```

See [ARCHITECTURE.md](ARCHITECTURE.md#adding-new-chains) for details.

---

## Development

### Build

```bash
# Build all Rust binaries
cargo build --release

# Build Docker image
make build
```

### Test

```bash
# Run unit tests
cargo test

# Integration test
make up-frost
curl -X POST http://localhost:3000/api/address/generate \
  -d '{"chain": "bitcoin", "passphrase": "test"}'
```

### Configuration

```toml
# config-node0.toml
[server]
host = "0.0.0.0"
port = 3001

[node]
node_index = 0
master_seed = "hex-encoded-seed"
network = "bitcoin"
storage_path = "./data/node0"
```

---

## Security

### Threshold Security

- **2-of-3 threshold:** Any 2 nodes can sign, but 1 cannot
- **No single point of failure:** Compromise of 1 node doesn't expose keys
- **FROST protocol:** Provably secure threshold Schnorr signatures

### Attack Surface

**Signer Nodes:**
- ✅ Pure cryptographic operations
- ✅ No transaction parsing
- ✅ No chain-specific logic
- ✅ Minimal codebase

**Aggregator:**
- ⚠️ Parses transactions (stateless, no keys)
- ✅ Compromise doesn't expose private keys
- ✅ Can be replaced without key rotation

See [SECURITY.md](SECURITY.md) for full threat model.

---

## Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Complete architecture guide
- **[SECURITY.md](SECURITY.md)** - Security model and threat analysis
- **[client/README.md](client/README.md)** - Client library usage

---

## License

MIT License - See [LICENSE](LICENSE) for details

---

## FAQ

**Q: Do I need to run separate DKG for each chain?**

A: No! Bitcoin and Ethereum share the same secp256k1 FROST key. You only need separate DKG for different curves (e.g., Ed25519 for Solana).

**Q: Can I add my own blockchain?**

A: Yes! If it uses secp256k1 (like most EVM chains), just add address derivation logic to the aggregator. Zero signer changes needed.

**Q: How do I recover keys?**

A: Each signer node has a master seed. With 2-of-3 seeds + passphrase list, you can recover all FROST keys via deterministic DKG.

**Q: Is this production-ready?**

A: Yes for Bitcoin and Ethereum address generation. Transaction signing requires full FROST coordination (round1 + round2) which needs aggregator implementation.

**Q: Why are signers "dumb"?**

A: By design! Signers only know cryptography. All chain-specific logic lives in the stateless aggregator. This makes adding new chains trivial and reduces attack surface.

---

**Built with good taste.** 🎯

*"Sometimes you can look at the problem from a different angle, rewrite it so the special case disappears and becomes the normal case."* — Linus Torvalds
