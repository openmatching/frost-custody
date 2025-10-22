# FROST Threshold Signatures with DKG

Bitcoin 2-of-3 threshold Schnorr signatures using FROST protocol with deterministic key generation.

## Overview

**FROST provides:**
- ✅ Per-user Taproot addresses (bc1p...)
- ✅ 56% smaller transactions (~110 vbytes vs ~250 vbytes)
- ✅ Better privacy (looks like normal wallet)
- ✅ Seed-recoverable shares (from master seeds + passphrases)

---

## How It Works

### Deterministic DKG Per Passphrase

**Each node has unique master seed (backup once):**
```
Node 0: master_seed_0 (from mnemonic)
Node 1: master_seed_1 (different mnemonic)
Node 2: master_seed_2 (different mnemonic)
```

**For each passphrase, nodes run DKG with deterministic RNG:**
```
Node 0: rng = ChaCha20(sha256(master_seed_0 + passphrase))
Node 1: rng = ChaCha20(sha256(master_seed_1 + passphrase))
Node 2: rng = ChaCha20(sha256(master_seed_2 + passphrase))

Run DKG protocol → share₀, share₁, share₂
Store in RocksDB (cache)
```

**Result:**
- Different passphrases → Different addresses
- Shares are deterministic (recoverable!)
- Real threshold security (each node only knows its seed)

---

## Deploy

```bash
make up-frost
```

**This starts:**
- 3 FROST signer nodes (internal, isolated)
- 1 FROST aggregator (port 5000, exposed to CEX)

---

## API

### Generate Address (Triggers DKG)

```bash
POST http://127.0.0.1:5000/api/address/generate
{
  "passphrase": "550e8400-e29b-41d4-a716-446655440000"
}

Response:
{
  "address": "bc1p...",
  "passphrase": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Process:**
1. Aggregator calls all 3 nodes for DKG round1
2. Nodes generate packages with deterministic RNG
3. Round2: Nodes create personalized packages
4. Finalize: Nodes store shares in RocksDB
5. Return unique Taproot address

### Sign Message

```bash
POST http://127.0.0.1:5000/api/sign
{
  "message": "deadbeef..."  # Bitcoin sighash
}
```

**Aggregator orchestrates 3-round FROST signing automatically.**

---

## Recovery

**What to backup:**
- 3 master seeds (mnemonics) ✅ Critical
- List of all passphrases (CEX database) ✅ Critical
- RocksDB databases ⚠️ Optional (can rebuild from #1 + #2)

**If RocksDB lost:**
```bash
# Re-run DKG for all passphrases
for passphrase in passphrase_list:
    POST /api/address/generate {passphrase}
    
# Rebuilds cache from master seeds
```

---

## vs Traditional Multisig

| Feature                       | Multisig    | FROST DKG                     |
| ----------------------------- | ----------- | ----------------------------- |
| **Size**                      | ~250 vbytes | ~110 vbytes                   |
| **Fee**                       | 12,500 sats | 5,500 sats (**56% cheaper**)  |
| **Annual cost** (1000 tx/day) | $2.7M       | $1.2M (**$1.5M savings**)     |
| **Recovery**                  | 3 mnemonics | 3 mnemonics + passphrase list |

**FROST saves $1.5M/year at scale!** 🚀

---

## See Also

- [frost-aggregator/README.md](frost-aggregator/README.md) - Aggregator details
- [frost-signer/README.md](frost-signer/README.md) - Signer technical details
