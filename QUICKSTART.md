# Quick Start Guide

## Test Locally (Development)

### 1. Copy Example Config

```bash
cp config.toml.example config.toml
```

### 2. Start Server

```bash
cargo run
```

The server will start on `http://localhost:3000` using the example configuration.

**Open interactive API docs**: http://localhost:3000/docs

### 3. Test APIs

**Get address for user 123:**
```bash
curl http://localhost:3000/api/address?id=123
```

**Get pubkey for user 456:**
```bash
curl http://localhost:3000/api/pubkey?id=456
```

**Health check:**
```bash
curl http://localhost:3000/health
```

## Production Setup

### Generate Real Keys

**Option 1: Using Hardware Wallet**
- Generate seed on hardware wallet
- Export xpub at path `m/48'/0'/0'/2'`
- Repeat for 3 devices

**Option 2: Using BIP39 Tool**
- Go to https://iancoleman.io/bip39/
- Generate 24-word mnemonic (offline!)
- Set derivation path to `m/48'/0'/0'/2'`
- Copy "Account Extended Public Key"
- Repeat 3 times

### Configure 3 Nodes

**Node 0: config-node0.toml**
```toml
[network]
type = "bitcoin"

[signer]
node_index = 0
mnemonic = "first twenty four word mnemonic here..."
xpubs = ["xpub0...", "xpub1...", "xpub2..."]

[server]
host = "0.0.0.0"
port = 3000
```

**Node 1: config-node1.toml** (same but `node_index = 1`, different mnemonic)

**Node 2: config-node2.toml** (same but `node_index = 2`, different mnemonic)

### Deploy with Docker

```bash
# Build image
docker build -t consensus-ring .

# Run all nodes
docker-compose up -d

# Check logs
docker-compose logs -f

# Test
curl http://localhost:3000/health
curl http://localhost:3001/health
curl http://localhost:3002/health
```

### Verify Setup

All three nodes should return the **same address** for the same user ID:

```bash
curl http://localhost:3000/api/address?id=999
curl http://localhost:3001/api/address?id=999
curl http://localhost:3002/api/address?id=999

# All three should return identical address
```

## CEX Integration

### Store xpubs in Your Backend

```python
# In your CEX backend config
SIGNER_NODES = [
    "http://10.0.1.10:3000",
    "http://10.0.1.11:3000",
    "http://10.0.1.12:3000"
]

# From the 3 nodes' /health endpoints
XPUBS = [
    "xpub6C...",  # Node 0
    "xpub6D...",  # Node 1
    "xpub6E..."   # Node 2
]
```

### Generate Deposit Address

```python
# When user requests deposit address
user_id = 12345
response = requests.get(f"{SIGNER_NODES[0]}/api/address?id={user_id}")
address = response.json()["address"]

# Store in database
db.execute(
    "INSERT INTO user_deposits (user_id, address) VALUES (?, ?)",
    (user_id, address)
)

# Show to user
return {"deposit_address": address}
```

### Sweep Funds

```python
# Build PSBT with your UTXOs
utxos = db.get_all_utxos()
psbt = build_consolidation_psbt(utxos, cold_wallet_address)
user_ids = [utxo.user_id for utxo in utxos]

# Sign with node 0
response = requests.post(
    f"{SIGNER_NODES[0]}/api/sign",
    json={"psbt": psbt, "derivation_ids": user_ids}
)
psbt = response.json()["psbt"]

# Sign with node 1 (now have 2-of-3)
response = requests.post(
    f"{SIGNER_NODES[1]}/api/sign",
    json={"psbt": psbt, "derivation_ids": user_ids}
)
psbt = response.json()["psbt"]

# Finalize and broadcast
tx = finalize_psbt(psbt)
txid = broadcast_transaction(tx)
```

## Security Checklist

- [ ] Each node has different mnemonic
- [ ] All nodes have same 3 xpubs in same order
- [ ] config.toml has `chmod 600` permissions
- [ ] Mnemonics backed up offline (steel plate in vault)
- [ ] Nodes on isolated network (no internet)
- [ ] Firewall rules: only CEX backend can reach nodes
- [ ] Monitoring/alerting configured
- [ ] Tested recovery from mnemonic backup

## Common Issues

**Different addresses from different nodes:**
- Check xpubs are identical and in same order on all nodes

**"Invalid mnemonic":**
- Verify mnemonic is valid BIP39 (use checksum validator)

**Can't derive address from xpub:**
- Ensure xpubs are at path `m/48'/0'/0'/2'` (not `m/48'/0'/0'/2'/0'`)

**PSBT signing fails:**
- Verify PSBT inputs use addresses derived from same xpubs
- Check witness_script is present in PSBT inputs

