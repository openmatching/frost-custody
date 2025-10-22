# Traditional Multisig Signer

2-of-3 P2WSH multisig PSBT signing service.

## Quick Start

```bash
# Copy example config
cp config.toml.example config.toml

# Edit with your mnemonic and 3 xpubs

# Run
cargo run --bin signer-node

# Test
curl 'http://127.0.0.1:3000/api/address?passphrase=550e8400-e29b-41d4-a716-446655440000'
```

## Docker

```bash
docker-compose up -d
```

## API

- `GET /api/address?passphrase={uuid}` - Generate multisig address
- `GET /api/pubkey?passphrase={uuid}` - Get node's pubkey
- `POST /api/sign` - Sign PSBT with passphrases
- `GET /health` - Node status

**Use UUIDs for passphrases (NOT sequential IDs!)**

See [../cex-client/README.md](../cex-client/README.md) for CEX integration.

