# FROST Custody Architecture

## Overview

FROST Custody is a **3-tier threshold signing system** that separates concerns between cryptographic operations, key generation, and transaction signing.

## Design Principles

1. **Chain-agnostic signer nodes** - Only handle curve operations (secp256k1, Ed25519)
2. **Chain-aware aggregators** - Handle blockchain-specific logic (Bitcoin, Ethereum, Solana)
3. **Client isolation** - Clients never talk to signer nodes directly

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│                          CLIENT                              │
│                   (Web, Mobile, CLI)                         │
└────────────────┬─────────────────┬──────────────────────────┘
                 │                 │
                 │                 │
         ┌───────▼──────┐  ┌──────▼──────┐
         │  Address     │  │  Signing    │
         │  Aggregator  │  │  Aggregator │
         │  (Port 9000) │  │  (Port 8000)│
         │              │  │             │
         │  - DKG       │  │  - FROST    │
         │  - Address   │  │  - Signing  │
         └───────┬──────┘  └──────┬──────┘
                 │                │
                 │                │
         ┌───────▼────────────────▼─────────┐
         │      Signer Node Network          │
         │   (Ports 3001-3003 / 4000)       │
         │                                   │
         │  Node 1    Node 2    Node 3      │
         │   ┌──┐      ┌──┐      ┌──┐      │
         │   │DB│      │DB│      │DB│      │
         │   └──┘      └──┘      └──┘      │
         │                                   │
         │  - Curve operations only          │
         │  - No chain knowledge             │
         │  - Passphrase-based storage       │
         └───────────────────────────────────┘
```

## Components

### 1. Signer Nodes (Tier 1)

**Purpose:** Pure cryptographic operations

**Port:** 3001-3003 (local) / 4000 (docker)

**Role:** `node`

**Responsibilities:**
- DKG (Distributed Key Generation)
- FROST signing protocol (3 rounds)
- Key storage by passphrase
- Curve operations: secp256k1, Ed25519

**Does NOT know:**
- Bitcoin/Ethereum/Solana
- Address formats
- Transaction formats
- Networks (mainnet/testnet)

**API Endpoints:**
```
POST /api/dkg/secp256k1/round1
POST /api/dkg/secp256k1/round2
POST /api/dkg/secp256k1/finalize
POST /api/dkg/ed25519/round1
POST /api/dkg/ed25519/finalize
POST /api/pubkey/query
POST /api/frost/secp256k1/round1
POST /api/frost/secp256k1/round2
POST /api/frost/secp256k1/aggregate
POST /api/frost/ed25519/round1
POST /api/frost/ed25519/round2
POST /api/frost/ed25519/aggregate
```

**Configuration:**
```toml
[server]
role = "node"
host = "0.0.0.0"
port = 4000

[node]
node_index = 0
storage_path = "/data/frost-node0"
max_signers = 5
min_signers = 2
```

---

### 2. Address Aggregator (Tier 2)

**Purpose:** Orchestrate DKG and generate addresses

**Port:** 9000

**Role:** `address`

**Risk Level:** LOW (only generates addresses, cannot sign)

**Responsibilities:**
- Orchestrate DKG across signer nodes
- Retrieve public keys from nodes
- Derive chain-specific addresses:
  - Bitcoin: P2TR (Taproot)
  - Ethereum: Keccak256
  - Solana: Ed25519 Base58

**API Endpoints:**
```
POST /api/address/generate
  {
    "passphrase": "550e8400-e29b-41d4-a716-446655440000",
    "chain": "bitcoin"
  }
  → Returns: address, public_key, key_id

GET /health
```

**Configuration:**
```toml
[network]
type = "bitcoin"

[server]
role = "address"
host = "0.0.0.0"
port = 9000

[aggregator]
signer_nodes = [
    "http://frost-node0:4000",
    "http://frost-node1:4000",
    "http://frost-node2:4000",
]
threshold = 2
```

---

### 3. Signing Aggregator (Tier 3)

**Purpose:** Orchestrate FROST threshold signing

**Port:** 8000

**Role:** `signer`

**Risk Level:** HIGH (can sign transactions - restrict access!)

**Responsibilities:**
- Orchestrate FROST signing across signer nodes
- Calculate sighashes for PSBTs
- Aggregate signature shares
- Add signatures to PSBTs

**API Endpoints:**
```
POST /api/sign/message
  {
    "passphrase": "550e8400-e29b-41d4-a716-446655440000",
    "message": "deadbeef..."
  }
  → Returns: signature, verified

POST /api/sign/psbt
  {
    "psbt": "cHNidP8B...",
    "passphrases": ["pass1", "pass2"]
  }
  → Returns: signed_psbt, signatures_added

GET /health
```

**Configuration:**
```toml
[network]
type = "bitcoin"

[server]
role = "signer"
host = "0.0.0.0"
port = 8000

[aggregator]
signer_nodes = [
    "http://frost-node0:4000",
    "http://frost-node1:4000",
    "http://frost-node2:4000",
]
threshold = 2
```

---

## Client Usage

### Generate Address

```rust
// Client → Address Aggregator (9000)
POST http://127.0.0.1:9000/api/address/generate
{
  "passphrase": "550e8400-e29b-41d4-a716-446655440000",
  "chain": "bitcoin"
}

// Response
{
  "address": "bc1p...",
  "public_key": "02...",
  "key_id": "..."
}
```

### Sign PSBT

```rust
// Client → Signing Aggregator (8000)
POST http://127.0.0.1:8000/api/sign/psbt
{
  "psbt": "cHNidP8B...",
  "passphrases": [
    "550e8400-e29b-41d4-a716-446655440000",
    "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
  ]
}

// Response
{
  "signed_psbt": "cHNidP8B...",
  "signatures_added": 2
}
```

**Important:** Clients NEVER call signer nodes directly!

---

## Deployment

### Docker Compose

```bash
# Start all FROST services
make up-frost

# Check health
make test-frost

# View logs
make logs-frost
```

### Port Mapping

| Service            | Port | Role    | Risk |
| ------------------ | ---- | ------- | ---- |
| Signer Node 0      | 3001 | node    | HIGH |
| Signer Node 1      | 3002 | node    | HIGH |
| Signer Node 2      | 3003 | node    | HIGH |
| Address Aggregator | 9000 | address | LOW  |
| Signing Aggregator | 8000 | signer  | HIGH |

### Network Security

```yaml
# In production:
# - Signer nodes: Internal network only
# - Address aggregator: Expose to clients (safe)
# - Signing aggregator: Restricted access (dangerous!)
```

---

## Example: Sign Bitcoin PSBT

```rust
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let address_agg = "http://127.0.0.1:9000";
    let signing_agg = "http://127.0.0.1:8000";

    // Step 1: Generate address
    let addr = generate_address(address_agg, "passphrase1").await?;
    println!("Address: {}", addr);

    // Step 2: Build PSBT
    let psbt = build_psbt(utxos, destination, fee)?;

    // Step 3: Sign via aggregator
    let signed = sign_psbt(signing_agg, &psbt, &["passphrase1"]).await?;
    println!("Signed PSBT: {}", signed);

    // Step 4: Broadcast
    broadcast_psbt(&signed)?;

    Ok(())
}
```

---

## Security Model

### Threat Separation

1. **Address Aggregator Compromise**
   - Attacker can: Generate addresses (low impact)
   - Attacker cannot: Sign transactions
   - Risk: LOW

2. **Signing Aggregator Compromise**
   - Attacker can: Request signatures (needs passphrases)
   - Attacker cannot: Forge signatures without threshold nodes
   - Risk: HIGH (restrict access)

3. **Signer Node Compromise (< threshold)**
   - Attacker can: Read one key share
   - Attacker cannot: Sign without threshold shares
   - Risk: MEDIUM

4. **Signer Node Compromise (≥ threshold)**
   - Attacker can: Sign transactions (if has passphrases)
   - Risk: CRITICAL

### Best Practices

1. **Passphrase Management**
   - Store passphrases in HSM or secure enclave
   - Never log passphrases
   - Use unique passphrase per user/wallet

2. **Network Segmentation**
   - Signer nodes: Internal network only
   - Address aggregator: DMZ (safe to expose)
   - Signing aggregator: VPN/firewall (restrict!)

3. **Threshold Configuration**
   - Minimum: 2-of-3 (standard)
   - Recommended: 3-of-5 (better security)
   - Enterprise: 5-of-7 (maximum security)

---

## Development

### Run Example

```bash
# Start services
make up-frost

# Run example
cargo run --example sign_psbt_frost

# View logs
make logs-frost
```

### Testing

```bash
# Unit tests
cargo test --workspace

# Linting
cargo clippy --workspace

# Health checks
make test-frost
```

---

## Summary

This architecture achieves:

✅ **Separation of concerns** - Crypto vs. blockchain logic  
✅ **Defense in depth** - Multiple layers of security  
✅ **Flexibility** - Support any chain (Bitcoin, Ethereum, Solana)  
✅ **Scalability** - Add nodes without changing clients  
✅ **Clean APIs** - Simple, well-defined interfaces  

The key insight: **Signer nodes know nothing about blockchains, aggregators know everything.**
