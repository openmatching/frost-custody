# AWS KMS Setup Guide

## Overview

AWS KMS (Key Management Service) is now supported as a key provider alternative to PKCS#11 HSMs.

**Benefits:**
- ✅ **Cheap** - $1/month per key + $0.03 per 10K requests
- ✅ **Managed** - AWS handles backups, redundancy, compliance
- ✅ **No hardware** - Cloud-native, no USB tokens or physical HSMs
- ✅ **Scalable** - Same interface works across unlimited nodes
- ✅ **IAM integrated** - Fine-grained access control via AWS IAM

**Trade-offs:**
- ⚠️ **Trust AWS** - You're trusting AWS won't access your keys
- ⚠️ **Network dependency** - Requires AWS API access (3-20ms latency)
- ⚠️ **Cloud lock-in** - Keys stay in AWS (cannot export)

## When to Use

|              | PKCS#11 HSM                | AWS KMS                      |
| ------------ | -------------------------- | ---------------------------- |
| **Cost**     | $55-$50K+                  | $1/month + usage             |
| **Control**  | Full (you own hardware)    | Shared (trust AWS)           |
| **Backup**   | Manual (you manage)        | Automatic (AWS managed)      |
| **Latency**  | <1ms (local)               | 3-20ms (network)             |
| **Setup**    | Complex (physical devices) | Simple (API calls)           |
| **Best for** | High-security, on-prem     | Cost-effective, cloud-native |

**Recommendation:** Use AWS KMS if you already trust AWS with your infrastructure and want managed key security.

---

## Quick Start

### 1. Create KMS Keys (One per Node)

```bash
# Node 0
aws kms create-key \
  --description "FROST MPC Node 0" \
  --key-spec ECC_NIST_P256 \
  --key-usage SIGN_VERIFY \
  --region us-east-1

# Save the KeyId from output:
# {
#   "KeyMetadata": {
#     "KeyId": "12345678-1234-1234-1234-123456789012",
#     ...
#   }
# }

# Create alias (optional but recommended)
aws kms create-alias \
  --alias-name alias/frost-node-0 \
  --target-key-id 12345678-1234-1234-1234-123456789012

# Repeat for node 1, node 2, etc.
```

### 2. Configure IAM Permissions

Create an IAM policy for each node:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "kms:DescribeKey",
        "kms:Sign"
      ],
      "Resource": "arn:aws:kms:us-east-1:123456789012:key/12345678-1234-1234-1234-123456789012"
    }
  ]
}
```

Attach to EC2 instance role or ECS task role.

### 3. Configure Node

**`config-node0.toml`:**
```toml
[network]
type = "mainnet"

[server]
role = "node"
host = "0.0.0.0"
port = 4000

[node]
index = 0
storage_path = "/data/frost-shares-node0"
max_signers = 3
min_signers = 2

[node.key_provider]
type = "aws-kms"
key_id = "arn:aws:kms:us-east-1:123456789012:key/12345678-1234-1234-1234-123456789012"
# Or use alias:
# key_id = "alias/frost-node-0"
```

### 4. Build with AWS KMS Feature

```bash
# Build with AWS KMS support (PKCS#11 is still default)
cargo build --release --features aws-kms

# Or make it the only feature
cargo build --release --no-default-features --features aws-kms
```

### 5. Deploy with AWS Credentials

**Option A: EC2 Instance Role (Recommended)**
```bash
# No environment variables needed
# IAM role attached to EC2 instance
./frost-service
```

**Option B: Environment Variables**
```bash
export AWS_ACCESS_KEY_ID=AKIA...
export AWS_SECRET_ACCESS_KEY=...
export AWS_DEFAULT_REGION=us-east-1

./frost-service
```

**Option C: AWS Credentials File**
```bash
# ~/.aws/credentials
[default]
aws_access_key_id = AKIA...
aws_secret_access_key = ...

# ~/.aws/config
[default]
region = us-east-1

./frost-service
```

---

## Security Considerations

### Key Rotation

AWS KMS keys **cannot be rotated for FROST MPC** because:
- FROST requires deterministic key derivation
- Same KMS key must produce same signatures
- Rotation would break address recovery

**Solution:** Create new keys and migrate on-chain (same as PKCS#11).

### Multi-Region Setup

For disaster recovery, enable multi-region keys:

```bash
aws kms create-key \
  --multi-region \
  --description "FROST Node 0 (Multi-Region)" \
  --key-spec ECC_NIST_P256
  
# Replicate to other regions
aws kms replicate-key \
  --key-id mrk-... \
  --replica-region eu-west-1
```

### Audit Logging

Enable CloudTrail to log all KMS operations:

```bash
aws cloudtrail create-trail \
  --name frost-kms-audit \
  --s3-bucket-name my-audit-bucket

aws cloudtrail start-logging \
  --name frost-kms-audit
```

---

## Cost Estimation

**Setup:**
- 3 nodes = 3 KMS keys = $3/month

**Usage (10,000 users, 1 tx/day):**
- DKG: 10,000 addresses × 3 nodes × 1 signature = 30,000 requests
- Signing: 10,000 tx/day × 3 nodes × 1 signature = 30,000 requests/day
- **Monthly:** ~900K requests = ~$2.70
  
**Total:** ~$6/month for 10K active users

Compare to:
- YubiKey: $55 one-time × 3 = $165
- Thales HSM: $5K-50K+ × 3 = $15K-150K+

---

## Performance

**Measured latency (us-east-1):**
- Same AZ: ~5ms per KMS call
- Cross-AZ: ~10ms per KMS call
- Cross-region: ~50-100ms

**Impact on address generation:**
- PKCS#11 (YubiKey): ~25ms
- AWS KMS (same region): ~35ms (+10ms)
- AWS KMS (cross-region): ~75ms (+50ms)

**Recommendation:** Deploy in same AWS region as your application.

---

## Troubleshooting

### "Failed to access AWS KMS key - check IAM permissions"

```bash
# Test KMS access
aws kms describe-key --key-id alias/frost-node-0

# If it works, check IAM role is attached to instance/task
aws sts get-caller-identity
```

### "No signature returned from AWS KMS"

Key must be `ECC_NIST_P256` with `SIGN_VERIFY` usage:

```bash
aws kms describe-key --key-id alias/frost-node-0 \
  | jq '.KeyMetadata | {KeySpec, KeyUsage}'

# Should show:
# {
#   "KeySpec": "ECC_NIST_P256",
#   "KeyUsage": "SIGN_VERIFY"
# }
```

### High Latency

Check region:

```bash
echo $AWS_DEFAULT_REGION

# Should match your application region
# Set in ~/.aws/config or environment
export AWS_DEFAULT_REGION=us-east-1
```

---

## Migration from PKCS#11

**Cannot migrate keys directly** - on-chain migration required.

1. Deploy new nodes with AWS KMS
2. Generate new addresses for new users
3. Migrate existing funds gradually:
   - Option A: Coordinate with users (withdraw-redeposit)
   - Option B: Sweep funds in batches

Same process as migrating between any HSM types.

---

## Comparison Matrix

|              | SoftHSM      | YubiKey    | AWS KMS       | Thales     |
| ------------ | ------------ | ---------- | ------------- | ---------- |
| **Cost**     | Free         | $55        | $1/mo + usage | $5K+       |
| **Setup**    | Software     | USB device | API calls     | Enterprise |
| **Security** | Testing only | Good       | Excellent     | Excellent  |
| **Backup**   | Manual       | Manual     | Automatic     | Manual     |
| **Multi-DC** | N/A          | Difficult  | Easy          | Expensive  |
| **Latency**  | <1ms         | <1ms       | 5-20ms        | <1ms       |
| **Scale**    | ∞            | 1 per node | ∞             | Limited    |

---

## Next Steps

1. Create KMS keys for each node
2. Configure IAM permissions
3. Update node configs (`type = "aws-kms"`)
4. Build with `--features aws-kms`
5. Test with `cargo xtask test-dkg` (requires AWS credentials)
6. Deploy to production

Questions? See main [README.md](README.md) or [SYSTEM_DESIGN.md](SYSTEM_DESIGN.md).

