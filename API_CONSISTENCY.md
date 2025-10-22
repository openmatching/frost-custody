# API Consistency Across All Components

## Unified Passphrase-Based API

All components now use **passphrases** (UUIDs) instead of sequential IDs.

### Traditional Multisig (signer-node)

```bash
GET /api/address?passphrase=550e8400-e29b-41d4-a716-446655440000
→ Returns: bc1q... (P2WSH multisig address)

POST /api/sign
{
  "psbt": "...",
  "passphrases": ["uuid1", "uuid2", ...]
}
```

### FROST Signer (frost-signer)

```bash
GET /api/address?passphrase=550e8400-e29b-41d4-a716-446655440000
→ Returns: bc1p... (P2TR Taproot address)

POST /api/frost/round1
{
  "message": "..."
}
```

### FROST Aggregator (frost-aggregator)

```bash
GET /api/address?passphrase=550e8400-e29b-41d4-a716-446655440000
→ Returns: bc1p... (P2TR Taproot address, proxied from frost-signer)

POST /api/sign
{
  "message": "..."
}
→ Orchestrates 3-round FROST, returns signature
```

---

## Passphrase Derivation

### Traditional Multisig

**Algorithm:** 9-level BIP32 derivation

```
Passphrase → SHA-256 → Split into 9 indices → m/i0/i1/.../i8
Each node derives: xpub.derive(path) → 3 pubkeys → multisig address
```

**Result:** bc1q... (P2WSH)

### FROST

**Algorithm:** Taproot tweaking

```
Passphrase → SHA-256 → Tweak scalar
Group pubkey + tweak → Tweaked pubkey → Taproot address
```

**Result:** bc1p... (P2TR)

---

## Consistency Guarantees

✅ **All components accept passphrases** (not IDs)  
✅ **Same passphrase → Same address** (per implementation)  
✅ **Different implementations → Different addresses** (multisig ≠ FROST)  
✅ **Deterministic** (reproducible)  
✅ **Secure** (256-bit space, no enumeration)  

---

## CEX Integration

**Your CEX backend only needs to know:**

1. **Use UUIDs as passphrases:**
```python
passphrase = str(uuid.uuid4())
```

2. **Store passphrase mapping:**
```sql
INSERT INTO deposits (user_id, passphrase, address) VALUES (?, ?, ?)
```

3. **Call appropriate service:**
```python
# For multisig
address = get("http://multisig:3000/api/address?passphrase={passphrase}")

# For FROST
address = get("http://aggregator:5000/api/address?passphrase={passphrase}")
```

**Consistent API across all components!** ✅

