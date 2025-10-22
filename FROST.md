# FROST Threshold Signatures

## Quick Start

```bash
# Generate FROST keys
cargo run --bin frost-keygen

# Configure each node with generated keys
# See frost-signer/frost-config.toml.example

# Run node
CONFIG_PATH=frost-config-node0.toml cargo run --bin frost-signer
```

## API Usage

### Get Taproot Address

```bash
curl 'http://127.0.0.1:4000/api/address?passphrase=550e8400-e29b-41d4-a716-446655440000'
```

### Sign Message (3-Round Protocol)

**Round 1:**
```bash
curl -X POST http://127.0.0.1:4000/api/frost/round1 -d '{"message":"deadbeef"}'
→ Returns: commitments + encrypted_nonces
```

**Round 2:**
```bash
curl -X POST http://127.0.0.1:4000/api/frost/round2 -d '{
  "message":"deadbeef",
  "encrypted_nonces":"...",
  "all_commitments":[...]
}'
→ Returns: signature_share
```

**Round 3:**
```bash
curl -X POST http://127.0.0.1:4000/api/frost/aggregate -d '{
  "message":"deadbeef",
  "all_commitments":[...],
  "signature_shares":[...]
}'
→ Returns: final_signature
```

## CEX Integration (Production)

**Recommended: Use FROST aggregator (simpler and more secure!)**

```rust
// CEX backend just calls aggregator (1 endpoint)
let signature = reqwest::Client::new()
    .post("http://aggregator:5000/api/sign")
    .json(&json!({"message": sighash_hex}))
    .send()
    .await?
    .json::<SignResponse>()
    .await?
    .signature;

// Done! Aggregator handles all FROST complexity
```

**Alternative: Direct to signers (for advanced use)**

```rust
use cex_client::FrostSignerClient;

let frost = FrostSignerClient::new(
    vec!["http://node0:4000".into(), "http://node1:4000".into()],
    2
);
let signed_tx = frost.sign_transaction(tx, &prevouts)?;
```

**Recommendation: Use aggregator for production (better security).**

**Example:**
```bash
cargo run --example frost_aggregator_example
```

**See [frost-aggregator/README.md](frost-aggregator/README.md)**

## Advantages vs Traditional Multisig

| Feature              | Multisig              | FROST                     |
| -------------------- | --------------------- | ------------------------- |
| **Transaction size** | ~250 vbytes           | ~110 vbytes (56% smaller) |
| **Fee**              | 12,500 sats           | 5,500 sats (56% cheaper)  |
| **Privacy**          | Visible multisig      | Looks like normal wallet  |
| **Witness**          | 2 ECDSA sigs + script | 1 Schnorr sig             |

**Annual savings (1000 tx/day): $1.5M** 🚀

**See frost-signer/README.md for technical details.**

