# Deployment Guide

## Quick Deploy

```bash
# Build Docker image (all 4 binaries)
make build

# Run FROST (recommended for production)
make up-frost

# Test
curl http://127.0.0.1:5000/health
```

**Done! CEX can now call `http://aggregator:5000` for signing.**

---

## Services Overview

| Service          | Entrypoint         | Port      | Exposed       | Purpose              |
| ---------------- | ------------------ | --------- | ------------- | -------------------- |
| multisig-node0-2 | `signer-node`      | 3000-3002 | Yes           | Traditional multisig |
| frost-node0-2    | `frost-signer`     | 4000-4002 | No (internal) | FROST signers        |
| frost-aggregator | `frost-aggregator` | 5000      | Yes           | FROST coordinator    |

**One image (`consensus-ring:latest`), different entrypoints!**

---

## Deployment Options

### Option A: FROST Only (Recommended)

```bash
make up-frost
```

**Starts:**
- 3 FROST signer nodes (internal network)
- 1 FROST aggregator (port 5000)

**CEX calls:** `http://aggregator:5000`

### Option B: Traditional Multisig Only

```bash
make up-multisig
```

**Starts:**
- 3 multisig nodes (ports 3000-3002)

**CEX calls:** `http://node0:3000`, `http://node1:3000`

### Option C: Both (For Migration)

```bash
make up-all
```

**Starts:**
- Everything (7 services)

**Use case:** Gradual migration from multisig to FROST

---

## Network Topology

```
CEX Backend
    ↓
    ├─→ multisig-node0:3000 (signer-net)
    ├─→ multisig-node1:3001 (signer-net)
    ├─→ multisig-node2:3002 (signer-net)
    │
    └─→ frost-aggregator:5000 (cex-network)
            ↓ (frost-internal, isolated!)
            ├─→ frost-node0:4000
            ├─→ frost-node1:4000
            └─→ frost-node2:4000
```

**Security:** FROST signers on isolated network, only aggregator can reach them.

---

## Makefile Commands

```bash
make build        # Build Docker image
make up-multisig  # Run traditional multisig
make up-frost     # Run FROST (recommended!)
make up-all       # Run everything
make down         # Stop all
make logs         # View logs
make logs-frost   # FROST aggregator logs
make test-frost   # Test FROST health
make clean        # Remove everything
```

---

## Production Checklist

Before deploying to production:

### 1. Generate Real Keys

```bash
# FROST keys
cargo run --bin frost-keygen
# Copy output into frost-config-node0.toml, node1.toml, node2.toml

# Traditional multisig (if using)
# Generate 3 BIP39 mnemonics
# Update config-node0.toml, node1.toml, node2.toml
```

### 2. Secure Configs

```bash
chmod 600 config-node*.toml
chmod 600 frost-config-node*.toml
chmod 600 aggregator-config.toml
```

### 3. Update aggregator-config.toml

```toml
[frost]
signer_nodes = [
    "http://frost-node0:4000",  # Use Docker service names
    "http://frost-node1:4000",
    "http://frost-node2:4000",
]
threshold = 2
```

### 4. Deploy

```bash
make build
make up-frost  # Or up-all
```

### 5. Verify

```bash
# Check health
curl http://localhost:5000/health

# Should show all 3 nodes healthy
```

---

## Monitoring

```bash
# View aggregator logs
make logs-frost

# View all logs
make logs

# Check service status
docker-compose ps
```

**Health endpoint shows individual node status:**
```json
{
  "status": "ok",
  "signer_nodes_total": 3,
  "signer_nodes_healthy": 3,
  "threshold": 2,
  "nodes": [
    {"url": "http://frost-node0:4000", "healthy": true},
    {"url": "http://frost-node1:4000", "healthy": true},
    {"url": "http://frost-node2:4000", "healthy": true}
  ]
}
```

---

## Troubleshooting

**Aggregator can't reach signers:**
```bash
# Check frost-internal network
docker network inspect consensus-ring_frost-internal

# Verify signers are running
docker-compose ps frost-node0 frost-node1 frost-node2
```

**CEX can't reach aggregator:**
```bash
# Check port mapping
docker-compose ps frost-aggregator

# Test from host
curl http://127.0.0.1:5000/health
```

---

## Summary

**Single unified deployment:**
- ✅ 1 Dockerfile
- ✅ 1 docker-compose.yml
- ✅ 1 Makefile
- ✅ 7 services from 1 image
- ✅ Different entrypoints per role

**Production-ready!** 🚀

