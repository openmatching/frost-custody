# Getting Started with Consensus Ring

## Choose Your Path

### For CEX Developers (Recommended)

**Start here:** [cex-client/README.md](cex-client/README.md)

This is your integration library. It shows how to:
- Derive addresses locally (fast!)
- Build PSBTs
- Sign with multisig or FROST
- Complete workflow

### For Infrastructure/DevOps

**Traditional Multisig:**
1. Read [signer-node/README.md](signer-node/README.md)
2. Run: `make up-multisig`
3. Test: `curl http://127.0.0.1:3000/health`

**FROST (Recommended):**
1. Read [frost-aggregator/README.md](frost-aggregator/README.md)
2. Run: `make up-frost`
3. Test: `curl http://127.0.0.1:5000/health`

### Understanding the Tech

- **[FROST.md](FROST.md)** - What is FROST? Why 56% cheaper?
- **[SECURITY.md](SECURITY.md)** - How passphrase derivation works
- **[COMPARISON.md](COMPARISON.md)** - vs MPC and other solutions

## Quickest Start

```bash
# 1. Clone and build
git clone <repo>
cd consensus-ring
make build

# 2. Run FROST (recommended)
make up-frost

# 3. Test
curl 'http://127.0.0.1:5000/health'

# 4. See CEX integration examples
cargo run --example derive_address
cargo run --example frost_aggregator_example
```

## Production Deployment

**Recommended architecture:**
```
CEX Backend → frost-aggregator (port 5000)
                     ↓
              frost-signer nodes (internal network)
```

**Why?**
- ✅ 56% fee savings vs traditional multisig
- ✅ Better privacy (Taproot)
- ✅ Signer nodes isolated from CEX
- ✅ Simple CEX integration (2 API endpoints)

**Read:** [frost-aggregator/README.md](frost-aggregator/README.md)

## Next Steps

1. ✅ Run examples
2. ✅ Read cex-client/README.md
3. ✅ Deploy frost-aggregator + frost-signer
4. ✅ Integrate into your CEX
5. ✅ Save millions in fees! 🚀

