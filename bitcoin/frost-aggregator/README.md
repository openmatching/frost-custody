# FROST Aggregator

Orchestration service for FROST threshold signing. Sits between your CEX backend and FROST signer nodes.

## Architecture

```
┌──────────────┐
│ CEX Backend  │ (Your application)
└──────┬───────┘
       │ HTTP (1 endpoint)
       ▼
┌──────────────────┐
│ FROST Aggregator │ (This service, port 5000)
│  - Orchestrates  │
│  - 3-round FROST │
└──────┬───────────┘
       │ Internal network only
       ├─────────┬─────────┐
       ▼         ▼         ▼
   ┌──────┐ ┌──────┐ ┌──────┐
   │Node 0│ │Node 1│ │Node 2│ (FROST signers, isolated)
   │:4000 │ │:4001 │ │:4002 │
   └──────┘ └──────┘ └──────┘
```

## Security Benefits

**Before (without aggregator):**
```
CEX → Knows all 3 signer URLs
CEX → Calls each signer directly
Attack surface: CEX compromise = access to all signers
```

**After (with aggregator):**
```
CEX → Only knows aggregator URL (1 endpoint)
Aggregator → Knows signer URLs (isolated)
Attack surface: CEX compromise ≠ signer access ✅
```

**Benefits:**
- ✅ CEX backend only exposed to 1 endpoint
- ✅ Signer nodes more isolated
- ✅ Easier to firewall/monitor
- ✅ CEX code simpler (no 3-round protocol)

## Quick Start

```bash
# Configure
cp aggregator-config.toml.example aggregator-config.toml
# Edit: set signer node URLs

# Run
cargo run --bin frost-aggregator

# Test
curl -X POST http://127.0.0.1:6000/api/sign \
  -H "Content-Type: application/json" \
  -d '{"message":"deadbeef..."}'
```

## API

### GET /api/address?passphrase={uuid}

Get Taproot address (proxies to signer node).

```bash
curl 'http://127.0.0.1:6000/api/address?passphrase=550e8400-e29b-41d4-a716-446655440000'
```

**Response:**
```json
{
  "passphrase": "550e8400-e29b-41d4-a716-446655440000",
  "address": "bc1p...",
  "script_type": "p2tr"
}
```

### POST /api/sign

Sign message with FROST threshold (orchestrates 3-round protocol automatically).

```bash
curl -X POST http://127.0.0.1:6000/api/sign \
  -H "Content-Type: application/json" \
  -d '{"message":"e3b0c442..."}'
```

**Response:**
```json
{
  "signature": "a1b2c3...",
  "verified": true,
  "signers_used": 2
}
```

### POST /api/sign/psbt

Sign Bitcoin PSBT with FROST threshold signatures (Taproot key-path spend).

Takes a PSBT with multiple inputs, signs each input's sighash with FROST using the corresponding passphrase, and returns the fully signed PSBT.

```bash
curl -X POST http://127.0.0.1:6000/api/sign/psbt \
  -H "Content-Type: application/json" \
  -d '{
    "psbt": "cHNidP8BAH...",
    "passphrases": ["user-deposit-550e8400", "user-deposit-6ba7b810"]
  }'
```

**Request:**
- `psbt`: Base64-encoded PSBT (unsigned)
- `passphrases`: Array of passphrases (one per input, in order)

**Response:**
```json
{
  "psbt": "cHNidP8BAH...",
  "inputs_signed": 2
}
```

**What it does:**
1. Decodes PSBT and validates inputs
2. Extracts sighash for each input (Taproot key-path)
3. For each input, performs FROST signing with the corresponding passphrase's key shares
4. Adds Schnorr signatures to PSBT
5. Returns fully signed PSBT ready for broadcast

**Example:** See `examples/sign_psbt_example.rs` for complete consolidation transaction flow

### GET /health

Health check with status of all signer nodes.

```bash
curl http://127.0.0.1:6000/health
```

**Response:**
```json
{
  "status": "ok",
  "signer_nodes_total": 3,
  "signer_nodes_healthy": 3,
  "threshold": 2,
  "nodes": [
    {"url": "http://frost-node0:4000", "healthy": true, "error": null},
    {"url": "http://frost-node1:4000", "healthy": true, "error": null},
    {"url": "http://frost-node2:4000", "healthy": true, "error": null}
  ]
}
```

**If degraded:**
```json
{
  "status": "degraded: only 1 of 3 nodes healthy (need 2)",
  "signer_nodes_total": 3,
  "signer_nodes_healthy": 1,
  "threshold": 2,
  "nodes": [
    {"url": "http://frost-node0:4000", "healthy": true, "error": null},
    {"url": "http://frost-node1:4000", "healthy": false, "error": "Connection error: ..."},
    {"url": "http://frost-node2:4000", "healthy": false, "error": "Connection error: ..."}
  ]
}
```

## CEX Integration

**Simple! Two endpoints:**

```rust
// 1. Get Taproot address
let address = reqwest::get(format!(
    "http://aggregator:5000/api/address?passphrase={}",
    uuid
))
.await?
.json::<AddressResponse>()
.await?
.address;

// 2. Sign transaction
let signature = reqwest::post("http://aggregator:5000/api/sign")
    .json(&json!({"message": sighash_hex}))
    .send()
    .await?
    .json::<SignResponse>()
    .await?
    .signature;

// Done! All FROST complexity hidden behind simple API
```

**CEX backend benefits:**
- ✅ Only 2 endpoints (address + sign)
- ✅ No FROST protocol knowledge needed
- ✅ Signer nodes completely isolated
- ✅ Simple error handling

**vs calling signers directly (complex):**
```rust
// Without aggregator - CEX must do this:
let r1_0 = call_node0_round1()?;
let r1_1 = call_node1_round1()?;
let r2_0 = call_node0_round2(r1_0, all_commitments)?;
let r2_1 = call_node1_round2(r1_1, all_commitments)?;
let sig = call_aggregate(commitments, shares)?;
// Complex and exposes all signer URLs to CEX!
```

## Configuration

```toml
[frost]
signer_nodes = [
    "http://10.0.1.10:4000",  # Internal IPs only
    "http://10.0.1.11:4000",
    "http://10.0.1.12:4000",
]
threshold = 2

[server]
host = "0.0.0.0"  # Exposed to CEX
port = 5000
```

**Network isolation:**
- FROST signers: Internal network only (10.0.x.x)
- Aggregator: Accessible to CEX backend
- CEX: Cannot reach signers directly ✅

## Deployment

```yaml
# docker-compose-frost-full.yml
version: "3.8"

services:
  # Signer nodes (isolated network)
  frost-node0:
    build: {context: ., args: {BINARY: frost-signer}}
    networks: [frost-internal]
    # No ports exposed!

  frost-node1:
    build: {context: ., args: {BINARY: frost-signer}}
    networks: [frost-internal]

  frost-node2:
    build: {context: ., args: {BINARY: frost-signer}}
    networks: [frost-internal]

  # Aggregator (bridge between CEX and signers)
  frost-aggregator:
    build: {context: ., args: {BINARY: frost-aggregator}}
    networks: [frost-internal, cex-network]
    ports: ["5000:5000"]

networks:
  frost-internal:
    internal: true  # No external access
  cex-network:
    # CEX backend connects here
```

**Security:**
- Signers on `frost-internal` (no external access)
- Aggregator bridges `frost-internal` and `cex-network`
- CEX only sees aggregator (port 5000)

## Summary

**FROST Aggregator simplifies and secures your architecture:**

| Aspect              | Without Aggregator     | With Aggregator          |
| ------------------- | ---------------------- | ------------------------ |
| **CEX knows**       | 3 signer URLs          | 1 aggregator URL         |
| **CEX calls**       | 5 API calls (3 rounds) | 1 API call               |
| **Attack surface**  | High (3 endpoints)     | Low (1 endpoint)         |
| **Code complexity** | CEX handles FROST      | Aggregator handles FROST |
| **Security**        | ⚠️ Signers exposed      | ✅ Signers isolated       |

**Use aggregator for production!** 🔒

---

## Examples

### Basic Message Signing

```bash
cargo run --example frost_aggregator_example
```

Demonstrates simple message signing with FROST threshold signatures.

### PSBT Signing (Recommended!)

```bash
cargo run --example sign_psbt_example
```

Complete end-to-end Bitcoin transaction signing flow:
- Generate 2 unique Taproot addresses via DKG
- Build consolidation PSBT (2 inputs → 1 output)
- Sign each input with passphrase-specific FROST shares
- Finalize transaction ready for broadcast

**This is what you'll use in production for consolidation transactions!**


