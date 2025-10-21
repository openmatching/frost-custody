# Design Decision: Why No Auto-Detection

## Problem Statement

Initially, we considered implementing automatic detection of which PSBT inputs a signer node can sign by:
1. Examining the witness_script in each PSBT input
2. Extracting the 3 multisig pubkeys
3. Scanning user_ids 0-9999 to find matches
4. Automatically signing matched inputs

## Decision: Auto-Detection Removed

**We removed auto-detection. The `derivation_ids` parameter is REQUIRED.**

## Reasons

### 1. Performance is Unacceptable

```
Worst case: 9999 derivations × 1000 inputs = ~10 million operations
Time: 50-100+ seconds for large consolidations
```

For a production CEX:
- Consolidations can have 1000+ inputs
- User IDs can be in millions
- 10k scan limit is arbitrary and inadequate
- Performance unpredictable (depends on user_id distribution)

### 2. CEX Must Track user_ids Anyway

CEX cannot avoid tracking user_ids because:

```rust
// CEX must build witness_script with 3 pubkeys
let witness_script = create_multisig_script(
    derive_pubkey(xpub0, user_id),  // ← Need user_id here!
    derive_pubkey(xpub1, user_id),
    derive_pubkey(xpub2, user_id)
);
```

**To build a PSBT, CEX must:**
1. Know which user owns each UTXO (for crediting deposits)
2. Derive the 3 pubkeys for that user_id
3. Build the witness_script

**If CEX already has user_ids, why scan 10k possibilities?**

### 3. False Sense of Simplicity

Auto-detection appears to simplify the API:

```json
// Looks simpler
{ "psbt": "..." }

vs

{ "psbt": "...", "derivation_ids": [...] }
```

But it's misleading because:
- CEX still needs all the same information
- CEX still must understand multisig completely
- Only difference: don't send user_ids to signer
- Cost: 100x slower performance

### 4. Production CEX Requirements

A real CEX needs:
- **Scalability**: Millions of users, not 10k
- **Predictability**: Fixed-time operations
- **Reliability**: No arbitrary limits
- **Performance**: Fast consolidations (1-2 seconds, not 60+)

Auto-detection fails all these criteria.

## What the API Requires

### Required Request

```json
POST /api/sign
{
  "psbt": "cHNidP8BAH...",
  "derivation_ids": [123, 456, 789]
}
```

- `derivation_ids[i]` = user_id for PSBT input `i`
- Array length must match PSBT inputs count
- Empty array = error

### Why This is Best

1. **Explicit**: Clear which user owns each input
2. **Fast**: O(n) where n = inputs, no scanning
3. **Scalable**: Works with billions of users
4. **Simple**: Straightforward implementation
5. **Predictable**: Constant time per input

## CEX Integration Pattern

```rust
// Step 1: Query UTXOs with user_ids (CEX already has this)
let utxos = db.query("
    SELECT txid, vout, amount, user_id 
    FROM utxos 
    WHERE unspent = true
")?;

// Step 2: Build PSBT and collect user_ids
let mut psbt = Psbt::new();
let mut derivation_ids = Vec::new();

for utxo in utxos {
    // Add input to PSBT
    psbt.add_input_with_witness_script(utxo, &xpubs)?;
    
    // Track user_id
    derivation_ids.push(utxo.user_id);  // ← Just one line!
}

// Step 3: Sign (fast!)
let signed = sign_psbt(&psbt, &derivation_ids)?;
```

**Adding `derivation_ids` is trivial - CEX already iterates through UTXOs!**

## Alternative Considered: BIP32 Derivation Info

Instead of scanning, PSBT could include BIP32 derivation paths:

```json
{
  "inputs": [{
    "bip32_derivation": {
      "<pubkey>": {
        "master_fingerprint": "...",
        "path": "m/48'/0'/0'/2/123"  // ← Contains user_id!
      }
    }
  }]
}
```

But this requires:
- CEX to add BIP32 metadata to PSBT
- More complex PSBT building
- Parsing derivation paths

**Simpler to just pass user_ids directly in API.**

## Conclusion

**Auto-detection trades significant complexity and performance penalties for minimal API convenience.**

The "convenience" is illusory because CEX must track user_ids anyway to build PSBTs.

**Final design:**
- ✅ Simple API: Just send user_ids array
- ✅ Fast: O(n) performance
- ✅ Scalable: No arbitrary limits
- ✅ Explicit: Clear ownership mapping
- ✅ Maintainable: Straightforward code

---

## Summary

| Aspect               | Auto-Detection              | Explicit derivation_ids |
| -------------------- | --------------------------- | ----------------------- |
| **API simplicity**   | Slightly simpler            | One extra parameter     |
| **Performance**      | 50-100+ seconds             | 1-2 seconds             |
| **Scalability**      | <10k users                  | Unlimited               |
| **CEX complexity**   | Same (still needs user_ids) | Same                    |
| **Code complexity**  | High (scanning logic)       | Low (straightforward)   |
| **Production ready** | ❌ No                        | ✅ Yes                   |

**Decision: Require explicit `derivation_ids` for clean, fast, production-ready design.**

