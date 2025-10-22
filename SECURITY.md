# Security Design

## Passphrase-Based Derivation (9-Level BIP32)

### The Solution

**Use 9 BIP32 derivation levels to get full 256-bit keyspace:**

```
Passphrase (UUID) → SHA-256 (256 bits) → Split into 9 chunks → BIP32 path

Path: m/48'/0'/0'/2'/i0/i1/i2/i3/i4/i5/i6/i7/i8
Each index i0-i8 is 24-28 bits (< 2^31, non-hardened)
```

### Security Properties

**1. No Birthday Paradox**
```
Keyspace: 2^256
Collision at 50%: 2^128 ≈ 10^38 users
Real CEX scale: < 10^9 users
Collision probability: < 10^-60 ✅
```

**2. No Enumeration Attacks**
```
UUID space: 2^128
Cannot guess other users' passphrases ✅
Attacker cannot enumerate all addresses ✅
```

**3. Standard BIP32**
```
CEX can derive addresses locally ✅
Compatible with any BIP32 library ✅
Hardware wallet compatible ✅
```

### Passphrase Recommendations

**✅ Good:**
```python
str(uuid.uuid4())  # "550e8400-e29b-41d4-a716-446655440000"
secrets.token_hex(32)  # Random hex
hashlib.sha256(f"{user_id}:{SECRET_SALT}").hexdigest()
```

**❌ Bad:**
```python
str(user_id)  # Sequential, guessable!
f"user_{user_id}"  # Predictable pattern
user_email  # May be known to attacker
```

## 2-of-3 Threshold Security

**Attack resistance:**
- 1 node compromised → ✅ Funds safe
- 2 nodes compromised → ❌ Funds at risk (accepted risk)

**FROST additional benefit:**
- Single key share reveals nothing about private key
- Traditional multisig: each key is complete (riskier if stolen)

## Encrypted Nonce Storage (FROST)

**Challenge:** FROST needs stateful nonces (nonce reuse = key exposure)

**Solution:** Encrypt nonces with node's key_package, return to client

**Security:**
- Nonces encrypted with node secret ✅
- Client cannot decrypt ✅
- Message-bound (prevents reuse) ✅
- Stateless servers ✅

**See frost-signer/README.md for details.**

