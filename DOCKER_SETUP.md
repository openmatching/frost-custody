# Docker Setup Guide

## Quick Start

The repository includes 3 pre-configured test config files ready for docker-compose:

```
config-node0.toml  → Node 0 (port 3000)
config-node1.toml  → Node 1 (port 3001)
config-node2.toml  → Node 2 (port 3002)
```

### Start All Nodes

```bash
docker-compose up -d
```

### Verify All Nodes

```bash
# Check all nodes return same address for user 123
curl http://localhost:3000/api/address?id=123
curl http://localhost:3001/api/address?id=123
curl http://localhost:3002/api/address?id=123

# All three should return identical address
```

### Test Signing Flow

```bash
# Get address from node 0
ADDRESS=$(curl -s http://localhost:3000/api/address?id=999 | jq -r .address)
echo "Multisig address: $ADDRESS"

# Check health of all nodes
curl http://localhost:3000/health
curl http://localhost:3001/health
curl http://localhost:3002/health
```

### View Logs

```bash
# All nodes
docker-compose logs -f

# Specific node
docker-compose logs -f node0
docker-compose logs -f node1
docker-compose logs -f node2
```

### Stop Nodes

```bash
docker-compose down
```

## Configuration Details

### Node 0 (config-node0.toml)
- **node_index**: 0
- **mnemonic**: `abandon abandon abandon...`
- **port**: 3000

### Node 1 (config-node1.toml)
- **node_index**: 1
- **mnemonic**: `zoo zoo zoo...`
- **port**: 3000 (mapped to host 3001)

### Node 2 (config-node2.toml)
- **node_index**: 2
- **mnemonic**: `legal winner thank...`
- **port**: 3000 (mapped to host 3002)

### Shared Configuration
All nodes share the same 3 xpubs:
```
xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brxMyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj
xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz
xpub6D4BDPcP2GT577Vvch3R8wDkScZWzQzMMUm3PWbmWvVJrZwQY4VUNgqFJPMM3No2dFDFGTsxxpG5uJh7n7epu4trkrX7x7DogT5Uv6fcLW5
```

This ensures all nodes derive the same multisig addresses for any given user ID.

## Production Deployment

⚠️ **NEVER use these test mnemonics in production!**

For production:

1. Generate 3 real mnemonics using a secure method
2. Derive xpubs at path `m/48'/0'/0'/2'` for each
3. Update config files with real mnemonics
4. Update xpubs array with the 3 derived xpubs
5. Secure config files: `chmod 600 config-node*.toml`
6. Use docker secrets or encrypted volumes
7. Deploy on isolated private network

### Recommended Production Setup

```yaml
# docker-compose.prod.yml
version: "3.8"

services:
  node0:
    build: .
    container_name: signer-node-0
    secrets:
      - node0_config
    environment:
      CONFIG_PATH: /run/secrets/node0_config
    networks:
      - signer-net
    restart: always

  node1:
    build: .
    container_name: signer-node-1
    secrets:
      - node1_config
    environment:
      CONFIG_PATH: /run/secrets/node1_config
    networks:
      - signer-net
    restart: always

  node2:
    build: .
    container_name: signer-node-2
    secrets:
      - node2_config
    environment:
      CONFIG_PATH: /run/secrets/node2_config
    networks:
      - signer-net
    restart: always

networks:
  signer-net:
    driver: bridge
    internal: true  # No external internet access

secrets:
  node0_config:
    file: ./secrets/config-node0.toml
  node1_config:
    file: ./secrets/config-node1.toml
  node2_config:
    file: ./secrets/config-node2.toml
```

## Troubleshooting

### Nodes return different addresses
- Check that all nodes have **identical xpubs array** in same order
- Verify xpubs are at correct path `m/48'/0'/0'/2'`

### Container fails to start
```bash
# Check logs
docker-compose logs node0

# Common issues:
# - Invalid mnemonic (check BIP39 checksum)
# - Port already in use
# - Config file syntax error
```

### Can't reach nodes
```bash
# Verify containers are running
docker-compose ps

# Check port mapping
docker ps | grep signer-node

# Test from inside container
docker exec -it signer-node-0 curl http://localhost:3000/health
```

## Network Security

For production isolated network:

```yaml
networks:
  signer-net:
    driver: bridge
    internal: true  # Disable external access
    ipam:
      config:
        - subnet: 172.20.0.0/16
          gateway: 172.20.0.1
```

Then configure CEX backend to reach nodes via internal IPs:
- Node 0: http://172.20.0.2:3000
- Node 1: http://172.20.0.3:3000
- Node 2: http://172.20.0.4:3000

