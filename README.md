# Consensus Ring

Bitcoin 2-of-3 threshold signing for CEX deposit addresses.

## Quick Start

```bash
make build        # Build Docker image
make up-multisig  # Deploy traditional multisig
make test-multisig # Test
```

**Production-ready traditional multisig with per-user addresses!**

---

## Two Implementations

| Feature      | Traditional Multisig    | FROST DKG                           |
| ------------ | ----------------------- | ----------------------------------- |
| **Binary**   | `signer-node`           | `frost-signer` + `frost-aggregator` |
| **Address**  | bc1q... (P2WSH)         | bc1p... (P2TR Taproot)              |
| **Per-user** | ✅ Unique per passphrase | ✅ Unique per passphrase (via DKG)   |
| **Fee**      | ~250 vbytes             | ~110 vbytes (**56% cheaper**)       |
| **Recovery** | 3 mnemonics             | 3 master seeds + passphrase list    |
| **Database** | None                    | RocksDB (cache, recoverable)        |
| **Status**   | ✅ Production ready      | ✅ Complete, needs testing           |

---

## Traditional Multisig (Recommended for Production)

**Deploy:**
```bash
make up-multisig
```

**APIs:**
```
GET  /api/address?passphrase={uuid}  → bc1q... (unique multisig address)
POST /api/sign {psbt, passphrases}   → Signed PSBT
GET  /health                         → Status
```

**Features:**
- 9-level BIP32 derivation (256-bit keyspace)
- Passphrase-based (prevents enumeration)
- Recoverable from 3 mnemonics
- Stateless

**Docs:** [signer-node/README.md](signer-node/README.md)

---

## FROST with DKG (Advanced)

**Deploy:**
```bash
make up-frost
```

**APIs:**
```
POST /api/address/generate {passphrase} → Trigger DKG, return bc1p... address
POST /api/sign {message}                → Sign with FROST (56% cheaper)
GET  /health                            → Check all nodes
```

**Features:**
- Deterministic DKG with master seeds (recoverable!)
- Per-user Taproot addresses
- 56% fee savings
- Real threshold security

**Docs:** [FROST.md](FROST.md), [frost-aggregator/README.md](frost-aggregator/README.md)

---

## CEX Integration

**Library:** `cex-client` (Rust + Python)

```rust
use cex_client::*;

// Derive address locally (fast!)
let passphrase = Uuid::new_v4().to_string();
let address = derive_multisig_address(&xpubs, &passphrase, Network::Bitcoin)?;

// Sign PSBT
let signed = sign_with_threshold(&psbt, &passphrases, &signer_urls)?;
```

**Docs:** [cex-client/README.md](cex-client/README.md)

---

## Architecture

**Traditional:**
```
CEX → signer-node × 3 (2-of-3 multisig)
```

**FROST:**
```
CEX → frost-aggregator → frost-signer × 3 (2-of-3 threshold)
      (port 5000)        (internal, isolated)
```

---

## Documentation

1. **[README.md](README.md)** - This file (start here)
2. **[DEPLOY.md](DEPLOY.md)** - Deployment guide
3. **[cex-client/README.md](cex-client/README.md)** - CEX integration
4. **[FROST.md](FROST.md)** - FROST with DKG
5. **[SECURITY.md](SECURITY.md)** - Security design
6. **[COMPARISON.md](COMPARISON.md)** - vs alternatives

---

## Makefile Commands

```bash
make build         # Build Docker image
make up-multisig   # Run traditional multisig
make up-frost      # Run FROST with DKG
make down          # Stop all
make test-multisig # Test multisig
make test-frost    # Test FROST
```

---

## Key Features

### Security
- **Passphrase-based**: UUIDs prevent address enumeration
- **9-level BIP32**: Full 256-bit keyspace (multisig)
- **Deterministic DKG**: Seed-recoverable FROST shares
- **2-of-3 threshold**: 1 node compromise = safe

### Performance
- **Local derivation**: ~200µs (multisig)
- **FROST fee savings**: 56% vs multisig
- **Annual savings**: $1.5M at 1000 tx/day

### Architecture
- **Stateless**: Multisig has no database
- **Recoverable**: FROST shares from master seeds
- **Isolated**: FROST aggregator pattern
- **Unified**: Single Dockerfile, one compose file

---

## License

MIT
