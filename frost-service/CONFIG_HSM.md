# Hardware Security Module (HSM) Support

## Overview

Protect master keys in hardware instead of plaintext config files using the **PKCS#11 industry standard**.

**Supported devices:** YubiKey, Nitrokey, Thales HSM, AWS CloudHSM, SoftHSM, or any PKCS#11-compliant device.

## Quick Setup

### 1. Install Dependencies

```bash
# For testing (SoftHSM)
sudo apt-get install softhsm2

# For YubiKey
sudo apt-get install yubikey-manager ykcs11

# For other devices: Install vendor PKCS#11 library
```

### 2. Generate Key

```bash
# SoftHSM (testing)
softhsm2-util --init-token --slot 0 --label "frost-node-0"
pkcs11-tool --module /usr/lib/softhsm/libsofthsm2.so --login \
  --keypairgen --key-type EC:prime256v1 --label "frost-node-0"

# YubiKey
yubico-piv-tool -s 9a -a generate -o public.pem
yubico-piv-tool -s 9a -a verify-pin -a selfsign-certificate \
  -S "/CN=frost-node-0/" -i public.pem -o cert.pem
yubico-piv-tool -s 9a -a import-certificate -i cert.pem

# Enterprise HSM: Use vendor tools
```

### 3. Configure

```toml
[node.key_provider]
type = "pkcs11"

# Pick your device library:
pkcs11_library = "/usr/lib/softhsm/libsofthsm2.so"        # SoftHSM
# pkcs11_library = "/usr/lib/libykcs11.so"                # YubiKey
# pkcs11_library = "/usr/lib/libnitrokey.so"              # Nitrokey  
# pkcs11_library = "/opt/nfast/toolkits/pkcs11/libcknfast.so"  # Thales
# pkcs11_library = "/opt/cloudhsm/lib/libcloudhsm_pkcs11.so"   # AWS

slot = 0
pin = "${HSM_PIN}"  # Read from environment
key_label = "frost-node-0"
```

See `config-pkcs11.toml.example` for full examples.

### 4. Build & Run

```bash
# Build with PKCS#11 support
cargo build --release --features pkcs11

# Run with PIN from environment
export HSM_PIN="your-pin"
cargo run --release --features pkcs11
```

## Cost Comparison (24 nodes)

| Device       | Total Cost    | Use Case                |
| ------------ | ------------- | ----------------------- |
| Plaintext    | $0            | Development only        |
| SoftHSM      | $0            | Testing only (software) |
| **YubiKey**  | **$2,640**    | **Small deployments**   |
| Thales HSM   | $120,000      | Enterprise              |
| AWS CloudHSM | $302,400/year | Cloud-native            |

## Comparison with Cobo/Fireblocks

| Feature     | FROST MPC (PKCS#11)    | Cobo/Fireblocks       |
| ----------- | ---------------------- | --------------------- |
| Standard    | ✅ PKCS#11 (any device) | ⚠️ Proprietary         |
| Self-Hosted | ✅ Yes                  | ❌ SaaS only           |
| Cost        | $2,640 (USB tokens)    | $$$$/month            |
| Key Control | ✅ 100% yours           | ⚠️ Split with provider |
| Open Source | ✅ Yes                  | ❌ Closed              |

## Unlock API (Recommended for Production)

**Don't store PIN in config - unlock via API instead.**

### Endpoints

**POST /api/hsm/unlock** - Unlock with PIN
```bash
curl -X POST http://localhost:4000/api/hsm/unlock -d '{"pin": "123456"}'
```

**POST /api/hsm/lock** - Lock (clear PIN from memory)
```bash
curl -X POST http://localhost:4000/api/hsm/lock
```

**GET /api/hsm/status** - Check status
```bash
curl http://localhost:4000/api/hsm/status
```

### Security Benefit

| PIN Location     | Machine Stolen                | Risk   |
| ---------------- | ----------------------------- | ------ |
| In config file   | Attacker reads file → has PIN | ❌ High |
| **Via API only** | HSM locked → useless          | ✅ Low  |

**Config:**
```toml
[node.key_provider]
type = "pkcs11"
pkcs11_library = "/usr/lib/libykcs11.so"
# pin field omitted - unlock via API
key_label = "frost-node-0"
```

See `config-pkcs11-nopin.toml.example` for complete example.

---

## FAQ

**Q: Works with any PKCS#11 device?**  
A: Yes. Just change `pkcs11_library` path.

**Q: Should I store PIN in config?**  
A: No (production). Use unlock API for security.

**Q: What happens if HSM is locked?**  
A: Operations fail with error. Call /api/hsm/unlock first.

**Q: What curve does HSM use?**  
A: P-256 (only for deriving randomness). FROST uses secp256k1/Ed25519 for blockchains.

**Q: SoftHSM for production?**  
A: No. Software-only, use for testing.

**Q: Migration from plaintext?**  
A: Generates new keys. Must migrate funds on-chain.

## Troubleshooting

```bash
# List devices
pkcs11-tool --module /path/to/library.so -L

# Test signing
echo "test" | pkcs11-tool --module /path/to/library.so \
  --login --sign --mechanism ECDSA-SHA256
```

## References

- PKCS#11: https://docs.oasis-open.org/pkcs11/
- YubiKey: https://developers.yubico.com/PIV/
- SoftHSM: https://www.opendnssec.org/softhsm/
