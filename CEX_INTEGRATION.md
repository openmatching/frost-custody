# CEX Integration Guide

## Question: Must CEX Know It's a Multisig Address?

**YES - CEX must know it's multisig when building PSBTs.**

Here's why and how:

---

## Part 1: Address Generation (CEX Must Know Multisig Structure)

When CEX generates a deposit address:

```rust
// CEX has the 3 account xpubs
let xpubs = [
    "xpub6EkTGi8Kh6bq...",  // Node 0
    "xpub6EV2WhLpxRVK...",  // Node 1
    "xpub6DyBA7T961cE...",  // Node 2
];

// CEX derives address for user 123
let user_id = 123;
let child_pubkeys = derive_child_pubkeys(xpubs, user_id);

// Create 2-of-3 multisig descriptor
let descriptor = wsh(sortedmulti(2, child_pubkeys[0], child_pubkeys[1], child_pubkeys[2]));
let address = descriptor.address();

// Result: bc1q...multisig...address
```

**CEX must know:**
- ✅ It's a 2-of-3 multisig (threshold = 2, total = 3)
- ✅ The 3 xpubs for all signing nodes
- ✅ The derivation path (m/48'/0'/0'/2/{user_id})

---

## Part 2: PSBT Creation (CEX Must Provide Multisig Info)

When CEX builds a consolidation PSBT, it must include the **witness_script** for each input.

### What is witness_script?

The witness_script is the actual multisig script:

```
OP_2 <pubkey_0> <pubkey_1> <pubkey_2> OP_3 OP_CHECKMULTISIG
```

This tells the signer:
- "This is a 2-of-3 multisig"
- "Here are the 3 public keys"
- "You need 2 signatures to spend"

### Building a PSBT (Detailed Example)

```rust
use bitcoin::psbt::Psbt;
use bitcoin::Script;

// Step 1: CEX queries its database for UTXOs to consolidate
let utxos = db.query("
    SELECT txid, vout, amount, user_id, address 
    FROM utxos 
    WHERE unspent = true
    LIMIT 1000
")?;

// Step 2: Create PSBT
let mut psbt = Psbt::from_unsigned_tx(Transaction {
    version: 2,
    lock_time: 0,
    input: vec![],
    output: vec![
        TxOut {
            value: total_amount - fee,
            script_pubkey: cold_wallet_address.script_pubkey(),
        }
    ],
})?;

// Step 3: Add each UTXO as an input with multisig info
for utxo in utxos {
    // Add the input
    psbt.unsigned_tx.input.push(TxIn {
        previous_output: OutPoint {
            txid: utxo.txid,
            vout: utxo.vout,
        },
        sequence: 0xffffffff,
        ..Default::default()
    });
    
    // CRITICAL: CEX must construct the witness_script
    // Derive the 3 pubkeys for this user_id
    let child_pubkeys = derive_child_pubkeys(&xpubs, utxo.user_id);
    
    // Sort them (for sortedmulti)
    let mut sorted_pubkeys = child_pubkeys.clone();
    sorted_pubkeys.sort();
    
    // Build the witness_script
    let witness_script = Builder::new()
        .push_int(2)  // Threshold
        .push_key(&sorted_pubkeys[0])
        .push_key(&sorted_pubkeys[1])
        .push_key(&sorted_pubkeys[2])
        .push_int(3)  // Total keys
        .push_opcode(OP_CHECKMULTISIG)
        .into_script();
    
    // Add PSBT input metadata
    psbt.inputs.push(Input {
        witness_utxo: Some(TxOut {
            value: utxo.amount,
            script_pubkey: Address::from_str(&utxo.address)?.script_pubkey(),
        }),
        witness_script: Some(witness_script),  // ← REQUIRED for multisig!
        ..Default::default()
    });
}
```

### Why witness_script is Required

**Without witness_script:**
```json
{
  "inputs": [
    {
      "witness_utxo": {...}
      // ❌ Missing witness_script - signer can't sign!
    }
  ]
}
```
- Signer doesn't know it's multisig
- Signer doesn't know which pubkeys to use
- Signing will fail

**With witness_script:**
```json
{
  "inputs": [
    {
      "witness_utxo": {...},
      "witness_script": "5221...ae"  // ← Contains the 3 pubkeys
    }
  ]
}
```
- ✅ Signer can extract the 3 pubkeys
- ✅ Signer can match them to user_ids (auto-detection)
- ✅ Signer can create proper signature

---

## Part 3: Building and Signing PSBT

**CEX must track derivation_ids:**

```rust
// Build PSBT and track user_ids
let mut psbt = Psbt::new();
let mut derivation_ids = Vec::new();

for utxo in utxos {
    psbt.add_input(utxo);
    derivation_ids.push(utxo.user_id);  // ← Track which user owns each input
}

// Sign with derivation_ids
let req = SignRequest {
    psbt: base64::encode(psbt.serialize()),
    derivation_ids,  // ← Required: map each input to its user_id
};

let signed = client.post("http://node0:3000/api/sign")
    .json(&req)
    .send()
    .await?
    .json::<SignResponse>()
    .await?;
```

---

## Part 4: What CEX Must Store

### Minimum Required (For Both Approaches)

```sql
CREATE TABLE config (
    xpub_node0 TEXT NOT NULL,  -- Node 0 account xpub
    xpub_node1 TEXT NOT NULL,  -- Node 1 account xpub
    xpub_node2 TEXT NOT NULL   -- Node 2 account xpub
);

CREATE TABLE user_deposits (
    user_id BIGINT PRIMARY KEY,
    deposit_address TEXT NOT NULL  -- The bc1q... address
);

CREATE TABLE utxos (
    txid TEXT NOT NULL,
    vout INT NOT NULL,
    amount BIGINT NOT NULL,
    user_id BIGINT NOT NULL,  -- Which user owns this UTXO
    address TEXT NOT NULL,
    unspent BOOLEAN DEFAULT TRUE,
    PRIMARY KEY (txid, vout)
);
```

### Why user_id is Required

CEX must store `user_id` in the `utxos` table because:
1. **Crediting deposits**: When UTXO appears, credit the correct user
2. **Building PSBT witness_script**: Need user_id to derive the 3 pubkeys
3. **Signing**: Must provide derivation_ids array to signer

---

## Summary: What CEX Must Know

| Aspect                               | Required?  | Why                                           |
| ------------------------------------ | ---------- | --------------------------------------------- |
| **3 account xpubs**                  | ✅ Required | To derive addresses and build witness_scripts |
| **It's multisig**                    | ✅ Required | To build correct PSBT structure               |
| **2-of-3 threshold**                 | ✅ Required | To build witness_script                       |
| **Derivation path**                  | ✅ Required | To derive child pubkeys                       |
| **user_id per UTXO**                 | ✅ Required | To build witness_script and sign PSBTs        |
| **Provide derivation_ids to signer** | ✅ Required | Signer needs user_id for each input           |

---

## Key Points

**For CEX integration:**

1. ✅ CEX must know multisig structure (no way around this)
2. ✅ CEX must build PSBT with witness_script (required for signing)
3. ✅ CEX must track user_id per UTXO (required for building PSBTs and signing)
4. ✅ CEX must provide `derivation_ids` array when calling signer (maps inputs to user_ids)

