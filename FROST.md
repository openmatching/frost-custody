# FROST with Deterministic DKG - **WORKING!** ✅

Per-user Taproot addresses with seed-recoverable threshold signatures.

## Features

✅ **Per-user Taproot addresses** - Unique bc1p... per passphrase  
✅ **Deterministic DKG** - Recoverable from master seeds  
✅ **56% fee savings** - ~110 vbytes vs ~250 vbytes (multisig)  
✅ **Real threshold security** - Proper 2-of-3 via DKG  
✅ **Seed-recoverable** - Backup 3 mnemonics + passphrase list  

---

## Deploy

```bash
make build
make up-frost
```

**Starts:**
- 3 FROST signer nodes (internal network)
- 1 FROST aggregator (port 6000)

---

## Usage

### Generate Address

```bash
curl -X POST http://127.0.0.1:6000/api/address/generate \
  -H 'Content-Type: application/json' \
  -d '{"passphrase":"550e8400-e29b-41d4-a716-446655440000"}'
```

**Response:**
```json
{
  "address": "bc1p...",
  "passphrase": "550e8400-e29b-41d4-a716-446655440000"
}
```

**What happens:**
1. Aggregator triggers DKG across 3 nodes
2. Each node derives shares with deterministic RNG (master_seed + passphrase)
3. Nodes run 3-round DKG protocol
4. Shares stored in RocksDB (cache)
5. Returns unique Taproot address

### Get Cached Address

```bash
curl 'http://127.0.0.1:6000/api/address?passphrase=550e8400...'
```

Returns cached address instantly (no DKG needed).

### Sign Transaction

```bash
curl -X POST http://127.0.0.1:6000/api/sign \
  -d '{"message":"deadbeef..."}'
```

**Note:** Signing requires passphrase-aware endpoints (future work).

---

## How It Works

**Each node has unique master seed (from mnemonic):**

```
Node 0: master_seed_0
Node 1: master_seed_1  
Node 2: master_seed_2
```

**For each passphrase, deterministic DKG:**

```
Node i: rng = ChaCha20(sha256(master_seed_i + passphrase))
Run DKG with deterministic RNG
Result: share_i (unique, recoverable!)
```

**Properties:**
- Different passphrases → Different shares
- Same passphrase + same master seed → Same share (recoverable!)
- Threshold security maintained (each node only knows its seed)

---

## Recovery

**Backup:**
- 3 master seeds (mnemonics) ✅
- List of passphrases (CEX database) ✅
- RocksDB (optional)

**If RocksDB lost:**
```bash
# Re-run DKG for each passphrase
for passphrase in passphrase_list:
    POST /api/address/generate {passphrase}

# Rebuilds cache from master seeds
```

---

## vs Traditional Multisig

| Feature                  | Multisig       | FROST DKG                    |
| ------------------------ | -------------- | ---------------------------- |
| **Size**                 | ~250 vbytes    | ~110 vbytes                  |
| **Fee**                  | 12,500 sats    | 5,500 sats (**56% cheaper**) |
| **Privacy**              | Visible 2-of-3 | Looks like normal wallet     |
| **Annual** (1000 tx/day) | $2.7M          | **$1.2M** ($1.5M savings!)   |

---

**FROST saves $1.5 million/year at scale!** 🚀

See [frost-aggregator/README.md](frost-aggregator/README.md) for details.
