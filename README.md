# Consensus Ring

Bitcoin multisig PSBT signing service for CEX deposit address management.

## Overview

- **2-of-3 multisig**: Deploy 3 nodes, any 2 can sign
- **Deterministic**: Each user gets unique address from `m/48'/0'/0'/2/{user_id}`
- **Stateless**: No database, keys derived on-demand from mnemonic
- **OpenAPI**: Auto-generated API documentation

## Quick Start

### 1. Generate Keys

Generate 3 mnemonics (one per node). Use any BIP39 tool or run:

```bash
# Install bip39 CLI tool
cargo install bip39

# Generate mnemonic
bip39 generate --words 24
```

For each mnemonic, derive the account xpub at path `m/48'/0'/0'/2'`. You can use:
- Ian Coleman's BIP39 tool: https://iancoleman.io/bip39/
- Hardware wallet
- Bitcoin Core with descriptor wallets

You'll need:
- 3 mnemonics (keep secret, one per node)
- 3 account xpubs at `m/48'/0'/0'/2'` (share with all nodes)

### 2. Configure

Copy example config:

```bash
cp config.toml.example config.toml
```

Edit `config.toml`:

```toml
[network]
type = "bitcoin"  # or "testnet"

[signer]
node_index = 0  # 0, 1, or 2 (different for each node)
mnemonic = "your twenty four word mnemonic phrase here from step one above for this specific node"
xpubs = [
    "xpub6C...",  # Node 0 xpub
    "xpub6D...",  # Node 1 xpub
    "xpub6E..."   # Node 2 xpub
]

[server]
host = "0.0.0.0"
port = 3000
```

### 3. Run

```bash
# Development
cargo run

# Production
cargo build --release
./target/release/consensus-ring
```

Server starts on `http://0.0.0.0:3000`

**Interactive API Documentation**: `http://localhost:3000/docs`  
**OpenAPI Spec**: `http://localhost:3000/spec`

## API

### GET /api/address?id={user_id}

Get 2-of-3 multisig address for user.

```bash
curl http://localhost:3000/api/address?id=123
```

Response:
```json
{
  "user_id": 123,
  "address": "bc1q...",
  "script_type": "wsh_sortedmulti(2,3)"
}
```

### GET /api/pubkey?id={user_id}

Get this node's public key for user.

```bash
curl http://localhost:3000/api/pubkey?id=123
```

Response:
```json
{
  "user_id": 123,
  "pubkey": "02a1b2c3...",
  "node_index": 0
}
```

### POST /api/sign

Sign PSBT with this node's keys.

```bash
curl -X POST http://localhost:3000/api/sign \
  -H "Content-Type: application/json" \
  -d '{
    "psbt": "cHNidP8BAH...",
    "derivation_ids": [123, 456, 789]
  }'
```

Response:
```json
{
  "psbt": "cHNidP8BAH...",
  "signed_count": 3,
  "node_index": 0
}
```

### GET /health

Health check.

```bash
curl http://localhost:3000/health
```

Response:
```json
{
  "status": "ok",
  "node_index": 0,
  "xpub": "xpub6C..."
}
```

## Docker Deployment

### Build

```bash
docker build -t consensus-ring .
```

### Run 3 Nodes

**Test configs are already included** (`config-node0.toml`, `config-node1.toml`, `config-node2.toml`) with example mnemonics.

```bash
# Start all 3 nodes
docker-compose up -d

# Check logs
docker-compose logs -f

# Check status
docker-compose ps

# Stop all nodes
docker-compose down
```

Nodes available at:
- Node 0: http://localhost:3000 (docs: http://localhost:3000/docs)
- Node 1: http://localhost:3001 (docs: http://localhost:3001/docs)
- Node 2: http://localhost:3002 (docs: http://localhost:3002/docs)

⚠️ **For production**: Replace the mnemonics in `config-node*.toml` with real keys and secure the files (`chmod 600`).

## CEX Integration

### Generate Deposit Address

```rust
// Call any node to get address
let address = reqwest::get(format!(
    "http://node0:3000/api/address?id={}", 
    user_id
))
.await?
.json::<AddressResponse>()
.await?
.address;

// Store in database
db.execute(
    "INSERT INTO deposits (user_id, address) VALUES (?, ?)",
    (user_id, address)
)?;
```

### Consolidation

```rust
// 1. Build PSBT with 1000 inputs
let utxos = db.get_pending_utxos()?;
let psbt = build_consolidation_psbt(utxos, cold_wallet)?;
let derivation_ids: Vec<u64> = utxos.iter().map(|u| u.user_id).collect();

// 2. Sign with node 0
let psbt = reqwest::post("http://node0:3000/api/sign")
    .json(&SignRequest { psbt, derivation_ids: derivation_ids.clone() })
    .send().await?
    .json::<SignResponse>().await?
    .psbt;

// 3. Sign with node 1 (now have 2-of-3)
let psbt = reqwest::post("http://node1:3000/api/sign")
    .json(&SignRequest { psbt, derivation_ids })
    .send().await?
    .json::<SignResponse>().await?
    .psbt;

// 4. Finalize and broadcast
let tx = finalize_psbt(psbt)?;
broadcast(tx)?;
```

See `examples/client.rs` for full example.

## Security Recommendations

### Production Setup

1. **Physical Isolation**: Run nodes on isolated network (no internet)
2. **File Permissions**: `chmod 600 config.toml` (protect mnemonic)
3. **HSM/Encrypted Storage**: Use hardware security module for mnemonics
4. **Separate Keys**: Each node has different mnemonic
5. **Regular Backups**: Store mnemonics in secure offline location
6. **Monitoring**: Alert on unauthorized API access
7. **Rate Limiting**: Add rate limits to prevent abuse

### Network Setup

```
┌─────────────┐
│ CEX Backend │ (online, has xpubs only)
└──────┬──────┘
       │ Private Network (no internet)
       │
   ┌───┴────┬────────┬────────┐
   │        │        │        │
┌──▼──┐  ┌──▼──┐  ┌──▼──┐  ┌──▼──┐
│Node0│  │Node1│  │Node2│  │ FW  │
└─────┘  └─────┘  └─────┘  └─────┘
```

## Performance

- **Address derivation**: ~200 µs
- **Sign 1 input**: ~1 ms  
- **Sign 1000 inputs**: ~1-2 seconds
- **Throughput**: 5,000+ addresses/sec

## Architecture

### Key Derivation

```
Master Seed (from mnemonic)
  ↓
m/48'/0'/0'/2' (account level, hardened)
  ↓
m/48'/0'/0'/2/{user_id} (address level, non-hardened)
```

Non-hardened final level allows CEX backend to derive addresses from xpub without private keys.

### Multisig Script

```
OP_2 <pubkey_0> <pubkey_1> <pubkey_2> OP_3 OP_CHECKMULTISIG
```

Wrapped in P2WSH (SegWit), sorted pubkeys for determinism.

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Run Example

```bash
cargo run --example client
```

### Check Lints

```bash
cargo clippy
```

## Troubleshooting

### "Invalid mnemonic"

Ensure your mnemonic is valid BIP39 (12 or 24 words).

### "Failed to derive child pubkey"

Check that xpubs are at correct path `m/48'/0'/0'/2'`.

### "PSBT signature verification failed"

Ensure all nodes use same xpubs in same order.

### "Address mismatch"

All nodes must have identical xpubs array order.

## License

MIT
