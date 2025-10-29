# FROST MPC Deployment & Threat Model

## Do You Need This?

**Most people don't.** Only use FROST MPC if you're managing 1000+ addresses programmatically.

| Your Situation                       | Solution                        | Why                                               |
| ------------------------------------ | ------------------------------- | ------------------------------------------------- |
| Personal holdings                    | Hardware wallet (Ledger/Trezor) | Simple, secure, battle-tested                     |
| Small team (2-10 people)             | Multisig (2-of-3, 3-of-5)       | Native blockchain support, no infrastructure      |
| **Exchange/Custodian (1000+ users)** | **FROST MPC**                   | Only option for programmatic + threshold security |

### When You MUST Use FROST MPC

**Problem:** Managing 50,000 user deposit addresses

**Why hardware wallets fail:**
- Can't generate 50K addresses on a Ledger
- Can't automate signing (manual confirmation required)

**Why multisig fails:**
- 50K users = 50K separate multisig wallets
- Complex backup (50K wallet configs)
- Large transactions (250 vB vs 110 vB)

**FROST MPC solution:**
- 1 passphrase per user = 1 unique address
- Backup: n HSM keys
- API-based signing (no manual intervention)
- Threshold security (m-of-n tolerance)
- Small transactions (same as single-sig)

**Use cases:** Exchanges, custodial wallets, payment processors, institutional DeFi

---

## Network Architecture

### 3-Tier Security Model

```
┌─────────────────────────────────────────────────┐
│ Tier 3: Address Aggregator (Port 9000)         │
│ - Generate addresses (LOW RISK)                 │
│ - Can expose publicly with rate limiting        │
└──────────────┬──────────────────────────────────┘
               │
┌──────────────▼──────────────────────────────────┐
│ Tier 2: Signing Aggregator (Port 8000)         │
│ - Sign transactions (HIGH RISK)                 │
│ - VPN/internal only, strict access control      │
└──────────────┬──────────────────────────────────┘
               │
┌──────────────▼──────────────────────────────────┐
│ Tier 1: Signer Nodes (CRITICAL)                │
│ - Node 0, Node 1, Node 2, ...                  │
│ - Each in separate private network              │
│ - No internet access, no cross-node talk        │
│ - Only accept inbound from aggregators          │
└─────────────────────────────────────────────────┘
```

**Key principle:** Signer nodes are completely isolated, aggregators orchestrate.

---

## Backup & Recovery

### What to Backup

**Critical (lose this = lose everything):**

1. **HSM Keys** - One per node (stored in PKCS#11 HSM)
   ```
   Node 0: [abandon abandon ... art]
   Node 1: [zoo zoo ... wrong]
   ...
   Node n-1: [verify verify ... castle]
   ```
   Store in: Hardware wallet, paper in safe, Shamir's Secret Sharing

2. **Threshold Config**
   ```toml
   max_signers = n   # Total nodes (e.g., 3, 5, 24)
   min_signers = m   # Threshold (e.g., 2, 3, 18)
   ```

3. **Passphrase**
   ```
   user_id → passphrase (any string, high-entropy recommended)
   12345 → 550e8400-...  # UUID
   67890 → a3f5b9c2...   # hex
   99999 → 3J98t1Wp...   # base58
   ```

### Recovery (Total Infrastructure Loss)

```bash
# 1. Deploy n nodes with original HSM keys
# 2. Regenerate all keys
for passphrase in all_passphrases:
    POST /api/address/generate {"passphrase": "$passphrase"}

# 3. Verify addresses match records
# 4. Resume operations
```

**Time:** ~1 hour for 10K users (DKG is fast: 30-100ms per address)

**Result:** Same addresses, same keys (deterministic regeneration)

---

## Threat Model

**Assumes m-of-n configuration (e.g., 18-of-24). Principles apply to any threshold.**

### Quick Reference

| Compromised                   | Generate Addresses? | Sign Txs? | Funds Safe?      | Action                 |
| ----------------------------- | ------------------- | --------- | ---------------- | ---------------------- |
| < m node **data**             | No                  | No        | ✅ Yes            | Monitor                |
| ≥ m node **data** (encrypted) | No                  | No        | ✅ Yes            | Monitor                |
| ≥ m (data + HSM + PIN)        | No                  | Yes*      | ⚠️ **Critical**   | Migrate funds          |
| < n **HSM keys**              | No                  | No        | ✅ Yes            | Audit only             |
| All n **HSM keys**            | Yes                 | Yes       | ❌ **Total loss** | Migrate immediately    |
| Address aggregator            | Yes                 | No        | ✅ Yes            | Low impact             |
| Signing aggregator            | No                  | Yes*      | ⚠️ High           | Depends on passphrases |

**\* Needs passphrases** - Safe if high-entropy (UUID), critical if sequential (1,2,3...)

**Key Insight:** RocksDB data is encrypted with keys derived from HSM signatures. 
Attacker needs **ALL THREE**: node data + HSM access + PIN to decrypt and use key shares.

### Key Insight: Defense in Depth

**Node Data = Encrypted RocksDB shares**
- Encrypted with AES-256-GCM
- Encryption key derived from HSM signature  
- **Compromising ≥m node data alone → USELESS** (encrypted!)
- Attacker needs: **data + HSM + PIN** for each of ≥m nodes

**HSM Keys = Root secrets**
- Stored in PKCS#11 hardware
- **Compromising all n HSM keys → can regenerate ANY passphrase's keys**
- Total compromise: affects all past and future keys

**Security layers:**
1. **Data layer**: Encrypted at rest (AES-256-GCM)
2. **HSM layer**: Keys in hardware (PIN-protected)
3. **Network layer**: Internal-only signer nodes
4. **Threshold**: Need ≥m nodes to operate

---

## Threat Scenarios

### 1. Few Nodes Compromised (< m)

**Example:** 5 nodes in 18-of-24 config

**Impact:** ❌ Safe (need 18 to sign)

**Action:** Monitor, audit compromised nodes

---

### 2. Threshold Nodes Compromised (≥ m)

**Scenario A: Only node data stolen (≥m servers)**

**Example:** 18+ nodes' RocksDB data in 18-of-24 config

**Impact:** ✅ **Safe** - Data is encrypted

**Attacker gets:**
- Encrypted FROST shares from 18+ nodes
- **Cannot decrypt** (needs HSM + PIN for each node)

**Cannot do:**
- Cannot decrypt shares (no HSM access)
- Cannot sign anything

---

**Scenario B: Data + HSM + PIN for ≥m nodes**

**Example:** Physical access to 18+ servers + their HSM devices + PINs

**Impact:** ⚠️ **Critical** - Can sign for known passphrases

**Attacker can:**
- Decrypt RocksDB shares (has HSM + PIN)
- Sign for passphrases in database
- Drain funds for known passphrases

**Cannot do:**
- Cannot generate NEW addresses (needs all n HSM keys)
- Cannot guess unknown passphrases (if high-entropy)

**Action depends on passphrase entropy:**

**High-entropy (UUID, 256-bit hex/base58):**
- Attacker can't guess → plan migration (not urgent)
- Monitor for unusual HSM activity

**Low-entropy (sequential: 1, 2, 3...):**
- Attacker CAN enumerate → **MIGRATE NOW**
- Freeze signing aggregator
- Move all funds immediately

**Reality:** Compromising data + HSM + PIN for ≥m nodes is MUCH harder than just stealing data files.

---

### 3. Some HSM Keys Compromised (< n)

**Example:** 20 of 24 seeds compromised

**Impact:** ❌ Safe (DKG needs ALL seeds)

**Why:** Missing even 1 seed → completely different keys

**Action:** Security audit, no migration needed

---

### 4. All HSM Keys Compromised (n/n)

**Impact:** ❌ **Total compromise**

**Attacker can:** Regenerate any passphrase's keys

**Action:** Race to migrate all funds before attacker does

**Prevention:**
- Store seeds in HSM or hardware wallets  
- Use Shamir's Secret Sharing (3-of-5 split per seed)
- Geographic distribution

---

### 5. Aggregators Compromised

**Address Aggregator:**
- Can generate addresses (privacy leak)
- Cannot sign transactions
- Low impact

**Signing Aggregator:**
- Can request signatures (needs passphrases)
- Critical if passphrases are guessable
- High impact

---

### 6. No Backup (Seeds Lost)

**Impact:** ⚠️ **Permanent fund loss**

**Reality:**
- Cannot regenerate keys
- Cannot sign transactions  
- All funds locked forever

**Prevention:** 
- Back up HSM keys
- Test recovery quarterly
- Multiple geographic locations

---

## Passphrase Security

**System accepts ANY string as passphrase.** No format required.

**But passphrase entropy is your SECOND defense layer.**

### Recommended (High-Entropy)

```python
# ✅ Use one of these

uuid.uuid4()                    # 128-bit UUID
secrets.token_hex(32)           # 256-bit hex
base58.b58encode(secrets.token_bytes(32))  # 256-bit base58
hashlib.sha256(f"{user_id}:{SECRET_SALT}")  # Salted hash
```

**Security:** 2^128 to 2^256 space → impossible to enumerate

### Dangerous (Low-Entropy)

```python
# ❌ NEVER in production

str(user_id)              # "12345" - attacker tries 1,2,3...
f"user_{user_id}"         # "user_12345" - still sequential  
f"wallet-{counter}"       # "wallet-1" - predictable
```

**Security:** Sequential → attacker can enumerate all users

### Why It Matters

If ≥m nodes compromised:

| Passphrase Type     | Can Attacker Sign? |
| ------------------- | ------------------ |
| High-entropy (UUID) | ❌ No (can't guess) |
| Low-entropy (1,2,3) | ✅ Yes (enumerate)  |

---

## Operational Guidelines

### Threshold Configuration

| Environment | Configuration        | Tolerance       |
| ----------- | -------------------- | --------------- |
| Development | 2-of-3               | 1 node can fail |
| Production  | 3-of-5 or 5-of-7     | 2-4 nodes       |
| Enterprise  | 10-of-15 or 18-of-24 | 5-8 nodes       |

**Rules:**
- `m ≤ 2n/3` (Byzantine fault tolerance)
- `n - m ≥ 2` (tolerate failures without losing signing)

### Monitoring

- Log all sign requests
- Alert on unusual patterns (rate/amount/destination)
- Geographic anomaly detection

### Incident Response

- Define migration thresholds
- Pre-approved cold wallet addresses
- User communication templates

### Testing

- Quarterly penetration tests
- Quarterly recovery drills (test seed restoration)

---

## Hardware Security (PKCS#11)

**PKCS#11 support enabled by default** - works with any compliant device.

**Supported:**
- SoftHSM ($0, testing) - `cargo xtask test-dkg`
- YubiKey ($55, USB token)
- Thales HSM ($5K+, enterprise)
- AWS CloudHSM ($1K/month, cloud)

**Config:**
```toml
[node.key_provider]
type = "pkcs11"
pkcs11_library = "/usr/lib/libykcs11.so"  # Device-specific
pin = "${HSM_PIN}"
key_label = "frost-node-0"
```

**Dockerfiles:**
- `Dockerfile` - Production (lean, ~150MB)
- `Dockerfile.softhsm` - Testing (with SoftHSM, ~200MB)

Setup: `frost-service/CONFIG_HSM.md` | Testing: `hsm/README.md`

---

## Summary

**Security:**
1. Threshold (need ≥m nodes)
2. Passphrase entropy (high-entropy)
3. Network isolation
4. HSM-backed keys (PKCS#11)
5. **Encrypted storage (AES-256-GCM at rest)**

**Encrypted RocksDB:**
- All key shares encrypted before storage (AES-256-GCM)
- Encryption key derived from HSM signature (deterministic)
- **Defense in depth: Even with ≥m nodes compromised, attacker needs HSM + PIN**
- Minimal HSM overhead (1 signature per passphrase)
- Encrypted data is useless without corresponding HSM access

**Limitations:**
- No key rotation (on-chain migration required)
- ≥m nodes compromised → emergency migration

**Flexibility:**
- Any m-of-n (2-of-3 to 18-of-24+)
- Any PKCS#11 device
- Encrypted storage always on (no performance penalty)
