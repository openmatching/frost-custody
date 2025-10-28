# HSM Testing (SoftHSM)

Test PKCS#11 integration without physical hardware.

## Quick Start

```bash
# Automated test with performance measurement
cargo xtask test-dkg --hsm

# Or manual
cd hsm
docker-compose up
curl -X POST http://localhost:9000/api/address/generate \
  -d '{"chain": "bitcoin", "passphrase": "test-uuid"}'
docker-compose down
```

## What's Here

```
hsm/
├── docker-compose.yml       # 3 nodes + 2 aggregators with SoftHSM
├── docker-entrypoint.sh     # Auto-initializes SoftHSM tokens
├── node-00/01/02.toml       # PKCS#11 node configs
└── aggregator.toml, signing.toml
```

**Two Dockerfiles:**
- `../Dockerfile` - Production (lean, no SoftHSM)
- `../Dockerfile.softhsm` - Testing (this directory uses this one)

## How It Works

**On container start:**
1. Entrypoint checks `USE_SOFTHSM=true`
2. Initializes SoftHSM token with unique label
3. Generates P-256 key
4. Starts frost-service with PKCS#11 config

**Each node:** Separate SoftHSM volume, isolated keys.

**Key caching:** SoftHSM keys persist in Docker volumes for reproducible testing.
- First run: Generates new keys (~15s init time)
- Subsequent runs: Reuses existing keys (~instant startup)
- Same keys = deterministic addresses (testing same passphrase gives same address)

**Local directories (gitignored):**
```
hsm/softhsm/node0/  # SoftHSM tokens
hsm/softhsm/node1/
hsm/softhsm/node2/
hsm/data/node0/     # FROST key shares
hsm/data/node1/
hsm/data/node2/
```

## Performance

| Mode           | Latency   | Overhead            |
| -------------- | --------- | ------------------- |
| Plaintext      | 30-100ms  | Baseline            |
| **SoftHSM**    | **~23ms** | **~0ms (measured)** |
| YubiKey (prod) | 80-150ms  | +50-100ms (est)     |

**Measured:** `cargo xtask test-dkg --hsm` → 23ms average (2-of-3 threshold)

**SoftHSM is software-only** → no hardware overhead, actually faster than network coordination.

## Config Example

```toml
[node.key_provider]
type = "pkcs11"
pkcs11_library = "/usr/lib/softhsm/libsofthsm2.so"
slot = 0
pin = "1234"
key_label = "frost-node-0"
```

For production HSM (YubiKey, Thales, AWS), see `../frost-service/CONFIG_HSM.md`.

## Key Persistence

**SoftHSM keys persist in local directories** (gitignored) - reproducible testing by default.

```bash
# Stop, KEEP keys (faster next run)
docker-compose down
cargo xtask test-dkg --hsm  # Reuses same keys

# Stop, DELETE keys (fresh start)  
docker-compose down
rm -rf softhsm data
cargo xtask test-dkg --hsm  # Generates new keys
```

**Directories created:**
- `hsm/softhsm/node0-2/` - SoftHSM tokens (inspect with `ls -la softhsm/`)
- `hsm/data/node0-2/` - FROST key shares
- Both gitignored automatically

**Benefits:**
- ✅ Reproducible (same passphrase → same address)
- ✅ Faster iterations (~2s vs ~15s)
- ✅ Easy inspection (local files, not hidden in Docker volumes)
- ✅ Simple cleanup (`rm -rf softhsm data`)

## Troubleshooting

```bash
# Check tokens
docker exec frost-signer-node-0-softhsm softhsm2-util --show-slots

# Check logs
docker logs frost-signer-node-0-softhsm

# Reset if corrupted
docker-compose down -v && docker-compose up
```

## Production

⚠️ **SoftHSM is for TESTING ONLY** (software simulation, no physical security).

For production:
- YubiKey: $55/node
- Thales HSM: $5K+/node
- AWS CloudHSM: $1K/month/node

See `../frost-service/CONFIG_HSM.md` for hardware HSM setup.
