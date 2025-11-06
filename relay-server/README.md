# FROST Relay Server

libp2p Circuit Relay v2 server for browser-to-browser WebRTC connections.

## Purpose

Enables browser nodes to:
1. Connect to relay for signaling
2. Reserve relay slots (listen on p2p-circuit)
3. Dial other browser nodes via relay
4. Exchange WebRTC SDP for direct connections
5. Disconnect from relay (after WebRTC established)

**Relay is only used for signaling, not for data transfer.**

## Build

```bash
cd relay-server
cargo build --release
```

## Run

```bash
# Development
RUST_LOG=info cargo run

# Production
./target/release/frost-relay-server
```

## Configuration

**Ports:**
- TCP 9090: For relay-to-relay connections
- TCP 9091: WebSocket for browser nodes
- TCP 443: Secure WebSocket (requires TLS cert)

**Limits:**
- Max reservations: 1024 concurrent
- Max circuits: 16 concurrent
- Max circuits per peer: 4
- Reservation duration: 1 hour

## Deploy

### 1. Build Release Binary

```bash
cargo build --release
```

### 2. Deploy to Server

```bash
# Upload to server
scp target/release/frost-relay-server user@relay.frost-wallet.io:/usr/local/bin/

# Create systemd service
sudo nano /etc/systemd/system/frost-relay.service
```

**systemd service:**
```ini
[Unit]
Description=FROST Wallet Relay Server
After=network.target

[Service]
Type=simple
User=frost
WorkingDirectory=/opt/frost-relay
ExecStart=/usr/local/bin/frost-relay-server
Restart=always
RestartSec=10
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
```

### 3. Start Service

```bash
sudo systemctl enable frost-relay
sudo systemctl start frost-relay
sudo systemctl status frost-relay
```

### 4. Configure Reverse Proxy (Nginx)

**For WSS (secure WebSocket):**

```nginx
server {
    listen 443 ssl http2;
    server_name relay.frost-wallet.io;

    ssl_certificate /etc/letsencrypt/live/relay.frost-wallet.io/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/relay.frost-wallet.io/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:9091;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 3600s;
    }
}
```

### 5. Get SSL Certificate

```bash
sudo certbot --nginx -d relay.frost-wallet.io
```

## Monitoring

**Check logs:**
```bash
sudo journalctl -u frost-relay -f
```

**Check connections:**
```bash
ss -tunlp | grep 9091
netstat -an | grep 9091
```

## Cost

**Relay server infrastructure:**
- VPS: $5-10/month (DigitalOcean, Hetzner)
- CPU: 1 core (relay is lightweight)
- RAM: 1GB
- Bandwidth: ~1GB/month (signaling only)

**Why it's cheap:**
- Relay only forwards signaling messages
- Direct WebRTC established after initial connection
- Low resource usage

## Security

**Built-in libp2p security:**
- ✅ Noise protocol encryption
- ✅ Peer authentication
- ✅ Rate limiting (max circuits)
- ✅ No data storage (stateless relay)

**Relay cannot:**
- ❌ Decrypt messages (end-to-end encryption)
- ❌ Impersonate peers (cryptographic identity)
- ❌ See FROST protocol data (encrypted)

**Relay only sees:**
- ✅ Which peers connect
- ✅ Connection timing
- ✅ Bandwidth usage

## Updating

```bash
# Pull latest code
git pull

# Rebuild
cargo build --release

# Restart service
sudo systemctl restart frost-relay
```

## Troubleshooting

**Relay not reachable:**
- Check firewall: `sudo ufw allow 9091/tcp`
- Check nginx config
- Check SSL certificate

**Connection limit reached:**
- Increase `max_reservations` in code
- Restart server

**High CPU usage:**
- Check for DDoS
- Add rate limiting
- Scale horizontally (multiple relays)

## Architecture

```
Browser A ──────→ Relay Server ←────── Browser B
                 (Public VPS)
                      ↓
              WebSocket signaling
                      ↓
         Browser A ←─── Direct P2P ───→ Browser B
              (Relay no longer used)
```

**Relay is temporary infrastructure for connection establishment.**
