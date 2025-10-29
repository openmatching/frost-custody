# Security Design

## Defense in Depth Layers

1. **Threshold cryptography** (2-of-3) - no single point of compromise
2. **Passphrase-based isolation** - each client gets unique keys  
3. **Network isolation** - signer nodes never talk to each other or internet
4. **Hardware security (PKCS#11)** - keys protected by HSM, unlock API prevents unauthorized use
5. **Encrypted storage (AES-256-GCM)** - key shares encrypted at rest in RocksDB

---

## Passphrase-Based Derivation

**Use UUIDs (not sequential IDs) for all address generation.**

### Why Passphrases?

**Sequential IDs (bad):**
```
User 1 → id=1 → address_1
User 2 → id=2 → address_2
...
Attacker can enumerate all addresses!
```

**UUIDs (good):**
```
User 1 → uuid="550e8400-..." → address_a
User 2 → uuid="6ba7b810-..." → address_b

Attacker cannot guess UUIDs (2^128 space)
```

### Multisig: 9-Level BIP32

**Algorithm:**
```
Passphrase → SHA-256 → Split into 9 chunks → m/i0/i1/.../i8
Each chunk < 2^31 (non-hardened)
Total: 256-bit keyspace
```

**Benefits:**
- ✅ Full 256-bit space (no birthday paradox)
- ✅ Standard BIP32 (CEX can derive locally)
- ✅ No enumeration attacks

### FROST: Deterministic DKG

**Algorithm:**
```
Node i: rng = ChaCha20(sha256(HSM_sign(passphrase)))
Run DKG with deterministic RNG
Result: Unique FROST shares per passphrase
```

**Benefits:**
- ✅ Per-user Taproot addresses
- ✅ Recoverable from HSM keys
- ✅ Real threshold security

---

## Threshold Security

**2-of-3 configuration:**
- 1 node compromised → ✅ Funds safe
- 2 nodes compromised → ❌ Funds at risk

**Accepted risk model for Bitcoin custody.**

---

## Backup Strategy

### Traditional Multisig
```
Backup: 3 BIP39 mnemonics
Recovery: Restore mnemonics → derive all addresses
```

### FROST DKG
```
Backup:
  - 3 HSM key backups
  - List of passphrases (CEX database)

Recovery:
  - Re-run DKG for each passphrase
  - Rebuild RocksDB cache
```

---

## Passphrase Recommendations

**✅ Good:**
```python
str(uuid.uuid4())  # "550e8400-e29b-41d4-a716-446655440000"
secrets.token_hex(32)
hashlib.sha256(f"{user_id}:{SECRET_SALT}").hexdigest()
```

**❌ Bad:**
```python
str(user_id)  # Sequential, guessable!
```
