# FROST Custody

Bitcoin 2-of-3 threshold signing service for CEX deposit addresses.

## Quick Start

```bash
make build          # Build Docker image
make up-multisig    # Deploy traditional multisig
# OR
make up-frost       # Deploy FROST with DKG (56% fee savings!)
```

---

## Project Structure

```
consensus-ring/
├── bitcoin/
│   ├── multisig-signer/    # Traditional 2-of-3 multisig
│   └── frost-service/       # FROST with deterministic DKG
├── client/                  # CEX integration library
├── Dockerfile              # Builds all binaries
├── docker-compose.yml      # All services
├── Makefile                # Easy deployment
└── docs...
```

---

## Two Implementations

| Feature      | Multisig           | FROST DKG                        |
| ------------ | ------------------ | -------------------------------- |
| **Address**  | bc1q... (P2WSH)    | bc1p... (Taproot)                |
| **Per-user** | ✅ Unique           | ✅ Unique                         |
| **Size**     | ~250 vbytes        | ~110 vbytes (**56% smaller**)    |
| **Fee**      | 12,500 sats        | 5,500 sats (**56% cheaper**)     |
| **Recovery** | 3 mnemonics        | 3 master seeds + passphrase list |
| **Status**   | ✅ Production ready | ✅ **Working!**                   |

**Annual savings with FROST: $1.5M** (at 1000 tx/day) 🚀

---

## Traditional Multisig

**Deploy:** `make up-multisig` (ports 3000-3002)

**API:**
```
GET  /api/address?passphrase={uuid}  → bc1q... multisig
POST /api/sign {psbt, passphrases}   → Signed PSBT  
```

**Features:**
- 9-level BIP32 derivation (256-bit keyspace)
- Passphrase-based (prevents enumeration)
- Stateless, no database
- Seed-recoverable from 3 mnemonics

---

## FROST with DKG

**Deploy:** `make up-frost` (port 6000)

**API:**
```
POST /api/address/generate {passphrase} → Trigger DKG, return bc1p...
POST /api/sign {message}                → Sign with FROST
GET  /health                            → Check all nodes
```

**Features:**
- Deterministic DKG with master seeds (recoverable!)
- Per-user Taproot addresses
- 56% fee savings vs multisig
- RocksDB cache (recoverable from seeds)

**Example:**
```bash
curl -X POST http://localhost:6000/api/address/generate \
  -H 'Content-Type: application/json' \
  -d '{"passphrase":"550e8400-e29b-41d4-a716-446655440000"}'

# Returns: {"address":"bc1p...","passphrase":"..."}
```

---

## CEX Integration

**Library:** `client/` (Rust + Python)

```rust
use client::*;

// Multisig: Derive locally (fast!)
let address = derive_multisig_address(&xpubs, &passphrase, Network::Bitcoin)?;

// FROST: Generate via aggregator
let address = reqwest::post("http://aggregator:6000/api/address/generate")
    .json(&json!({"passphrase": passphrase}))
    .send().await?
    .json::<AddressResponse>().await?
    .address;
```

See [client/README.md](client/README.md)

---

## Makefile Commands

```bash
make build         # Build Docker image
make up-multisig   # Run multisig (ports 3000-3002)
make up-frost      # Run FROST (port 6000)
make up-all        # Run everything
make down          # Stop all
make test-multisig # Test multisig
make test-frost    # Test FROST
make clean         # Remove everything
```

---

## Key Innovations

1. **9-Level BIP32** - Full 256-bit keyspace with standard BIP32
2. **Passphrase Security** - UUIDs prevent enumeration attacks
3. **Deterministic DKG** - FROST shares recoverable from master seeds!
4. **FROST Aggregator** - Isolates signers from CEX backend

---

## Documentation

**Essential:**
- [README.md](README.md) - This file
- [FROST.md](FROST.md) - FROST with DKG guide
- [DEPLOY.md](DEPLOY.md) - Deployment guide
- [SECURITY.md](SECURITY.md) - Security design

**Components:**
- [client/README.md](client/README.md) - CEX integration
- [bitcoin/multisig-signer/README.md](bitcoin/multisig-signer/README.md)
- [bitcoin/frost-service/README.md](bitcoin/frost-service/README.md)

---

## Deployment

**Production-ready with either implementation:**

```bash
# Traditional (battle-tested)
make build && make up-multisig

# FROST (56% cheaper, modern)
make build && make up-frost
```

**Both work perfectly!** 🚀

---

## License

MIT
