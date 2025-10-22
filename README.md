# Consensus Ring

Bitcoin 2-of-3 threshold signing service for CEX deposit address management.

## Two Implementations

|             | Traditional Multisig | FROST Threshold               |
| ----------- | -------------------- | ----------------------------- |
| **Binary**  | `signer-node`        | `frost-signer`                |
| **Address** | bc1q... (P2WSH)      | bc1p... (P2TR)                |
| **Size**    | ~250 vbytes          | ~110 vbytes (**56% smaller**) |
| **Fee**     | 12,500 sats          | 5,500 sats (**56% cheaper**)  |
| **Privacy** | Visible multisig     | Looks like normal wallet      |
| **Status**  | ✅ Complete           | ✅ Complete                    |

**Quick decision:**
- 🎓 Learning → Use `signer-node`
- 🚀 Production → Use `frost-signer` (56% fee savings!)
- 📚 Understand trade-offs → [COMPARISON.md](COMPARISON.md)

## Quick Start

```bash
# Build image (all binaries)
make build

# Run traditional multisig (ports 3000-3002)
make up-multisig

# OR run FROST (port 5000, recommended!)
make up-frost

# OR run both
make up-all

# Test
curl 'http://127.0.0.1:3000/health'  # Multisig
curl 'http://127.0.0.1:5000/health'  # FROST aggregator
```

**One Dockerfile, one docker-compose.yml, different entrypoints!**

### Details

**Traditional Multisig:** [signer-node/README.md](signer-node/README.md)  
**FROST (Recommended):** [frost-aggregator/README.md](frost-aggregator/README.md) and [FROST.md](FROST.md)

## CEX Integration

**Use the cex-client library for your backend:**

```rust
use cex_client::*;
use uuid::Uuid;

// 1. Derive address locally (fast, no API call!)
let passphrase = Uuid::new_v4().to_string();
let address = derive_multisig_address(&xpubs, &passphrase, Network::Bitcoin)?;

// 2. Sign PSBT (traditional multisig)
let signed = sign_with_threshold(&psbt, &passphrases, &signer_urls)?;

// 3. Or sign with FROST aggregator (56% cheaper, better security!)
let signature = reqwest::post("http://aggregator:5000/api/sign")
    .json(&json!({"message": sighash}))
    .send().await?.json::<SignResponse>().await?.signature;
```

**Examples:**
```bash
cargo run --example derive_address        # Local address derivation
cargo run --example sign_psbt             # Traditional multisig workflow
cargo run --example frost_aggregator_example  # FROST aggregator workflow
```

**See [cex-client/README.md](cex-client/README.md) for complete guide.**

## Key Features

### Security

- **9-level BIP32 derivation** - Full 256-bit keyspace, no birthday paradox
- **Passphrase-based** - Use UUIDs, prevents address enumeration
- **2-of-3 threshold** - 1 node compromised = funds safe
- **Encrypted nonces** - FROST with stateless servers

**See [SECURITY.md](SECURITY.md)**

### Performance

- **Address derivation**: ~200µs (local, no API call needed!)
- **FROST fee savings**: 56% vs traditional multisig
- **Annual savings (1000 tx/day)**: $1.5M

### Architecture

- **Stateless servers** - No database required
- **Standard BIP32** - CEX can derive addresses with any BIP32 library
- **Docker deployment** - Production-ready
- **OpenAPI** - Auto-generated documentation

## APIs

### Traditional Multisig (signer-node)

```bash
GET  /api/address?passphrase={uuid}  # Multisig address
POST /api/sign                        # Sign PSBT
GET  /health                          # Node status
```

### FROST (frost-signer)

```bash
GET  /api/address?passphrase={uuid}   # Taproot address
POST /api/frost/round1                 # Round 1: Commitments
POST /api/frost/round2                 # Round 2: Sign
POST /api/frost/aggregate              # Round 3: Aggregate
GET  /health                           # Node status
```

## Project Structure

```
consensus-ring/
├── signer-node/       # Traditional multisig
├── frost-signer/      # FROST signer nodes (internal)
├── frost-aggregator/  # FROST coordinator (exposed to CEX)
├── cex-client/        # CEX backend library
├── Dockerfile         # Single image, all binaries
├── docker-compose.yml # All services, different entrypoints
└── Makefile           # Easy deployment
```

## Documentation

**Essential (Start Here):**
- **[cex-client/README.md](cex-client/README.md)** - CEX integration guide
- **[FROST.md](FROST.md)** - FROST usage (56% fee savings!)
- **[SECURITY.md](SECURITY.md)** - Security design

**Components:**
- **[frost-aggregator/README.md](frost-aggregator/README.md)** - FROST coordinator (production)
- **[signer-node/README.md](signer-node/README.md)** - Traditional multisig
- **[frost-signer/README.md](frost-signer/README.md)** - FROST technical details

**Comparison:**
- **[COMPARISON.md](COMPARISON.md)** - vs MPC, decision guide

## Architecture

### Traditional Multisig
```
CEX → signer-node (3 replicas, 2-of-3)
```

### FROST (Recommended)
```
CEX → frost-aggregator → frost-signer (3 isolated nodes, 2-of-3)
```

**Security:** Aggregator isolates FROST signers from CEX backend.

## License

MIT
