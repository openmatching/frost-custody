# FROST MPC Wallet - Chrome Extension

P2P multi-party computation wallet using FROST threshold signatures.

## Tech Stack

**Pure Rust (100%):**
- **UI**: Dioxus + Tailwind CSS
- **P2P**: libp2p with Circuit Relay v2 + DCUtR (direct P2P after connection)
- **Crypto**: FROST (secp256k1)
- **Storage**: Chrome Storage API
- **Relay Server**: libp2p relay for connection establishment (does not forward messages)

**No JavaScript runtime dependencies.**

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install wasm-pack
cargo install wasm-pack

# Add WASM target
rustup target add wasm32-unknown-unknown
```

### Build

```bash
cd extension
./build.sh
```

### Install in Chrome

1. Open `chrome://extensions`
2. Enable "Developer mode"
3. Click "Load unpacked"
4. Select `extension/dist/`

## Current Status

### ✅ Phase 1: Foundation Complete

- Chrome extension (side panel UI)
- libp2p compiles in WASM
- Peer ID generation working
- UI wizard flow complete
- Pure Rust stack (no JavaScript)

### 🎯 Current Milestone: DKG Working

**Goal:** Create wallet by running FROST DKG across multiple browser tabs.

**See [MILESTONE.md](MILESTONE.md) for detailed task breakdown.**

**Next tasks:**
1. Complete FROST DKG implementation
2. Wire gossipsub for message passing
3. Coordinate DKG rounds over P2P
4. Store wallet (KeyPackage + group public key)

### 📋 Future Milestones

- Message authentication (whitelist + signatures)
- Transaction signing (FROST over P2P)
- Reconnection & persistence
- Custody nodes with policy-based signing
- BIP39 backup & WebAuthn encryption

## Architecture

```
Extension (Pure Rust)
├── UI (Dioxus)
│   ├── Welcome screen
│   ├── Wallet creation wizard
│   ├── Transaction signing
│   └── Settings
├── P2P (libp2p)
│   ├── Circuit Relay client ✅
│   ├── DCUtR (direct connection upgrade) ✅
│   ├── Gossipsub messaging ✅
│   └── WebSocket transport ✅
├── Crypto (FROST)
│   ├── DKG (partial)
│   └── Signing (partial)
└── Storage (Chrome API)
    └── Key packages (partial)

Relay Server (Rust)
└── Connection establishment only (no message forwarding)

Custody Server (Rust)
└── Policy-based co-signer (planned)
```

## Key Design Decisions

### 1. Pure Rust P2P (libp2p)

**Fix:** Enable `wasm_js` feature for getrandom
```toml
getrandom = { version = "0.3", features = ["wasm_js"] }
```

**Bundle:** 563KB WASM, 1.1MB total

### 2. P2P Architecture: WebRTC Direct P2P

**Transport Strategy:**
- **WebRTC ONLY**: Both relay server and browsers use WebRTC (UDP-based)
- **No WebSocket**: Cleaner, faster, better NAT traversal

**Phase 1: Initial Connection**
```
Browser A ──WebRTC(UDP)──> Relay Server <──WebRTC(UDP)── Browser B
                      (Gossipsub mesh via relay)
```

**Phase 2: DCUtR Upgrade (Optional)**
```
Browser A ◀──────WebRTC Direct P2P──────▶ Browser B
              (Gossipsub now direct)
              Relay exits topology! ✅
```

**How it works (matches [js-libp2p pattern](https://github.com/libp2p/js-libp2p-example-webrtc-private-to-private)):**
1. Both browsers connect to relay via WebSocket
2. Coordinator listens on `/p2p-circuit` (circuit relay address)
3. Follower dials coordinator via `/p2p-circuit/p2p/COORD_ID`
4. Circuit relay establishes initial connection through relay
5. DCUtR upgrades to direct WebRTC browser-to-browser
6. DKG/signing messages flow over direct connection (relay NOT in path!)

**Benefits:**
- ✅ **WebRTC everywhere** - Same transport for relay and P2P
- ✅ **Auto peer discovery** - Gossipsub handles it automatically  
- ✅ **DCUtR upgrade** - Optional direct connection optimization
- ✅ **Better NAT traversal** - UDP-based WebRTC handles most firewalls


### 3. Coordinator/Follower Pattern

- One device coordinates (initiates actions)
- Other devices follow (respond to coordinator)
- Clear UI flow for users

### 4. Policy-Based Custody Nodes

Custody nodes have configurable signing policies:

```rust
enum SigningPolicy {
    AlwaysSign,              // Backup only
    AutoSignUnder(u64),      // Amount limit
    ManualApproval,          // Never auto-sign
}
```

**Example: 3-of-4 with tiered security:**
- User: 2 devices
- Custody: 2 nodes
  - Node 1: Auto-sign < $100 (daily spending)
  - Node 2: Auto-sign < $10k (large purchases)

**Scenarios:**
- Coffee ($5): Device + Node1 + Node2 = 3 signatures (auto)
- Car ($7k): Device + Device + Node2 = 3 signatures (both devices needed)
- House ($500k): Both devices + manual approval (max security)

## Project Structure

```
extension/
├── Cargo.toml           # Rust dependencies
├── manifest.json        # Chrome extension config
├── build.sh             # Build script
├── src/
│   ├── lib.rs          # Main app
│   ├── components/     # UI components
│   │   ├── create_wallet.rs
│   │   ├── wallet_view.rs
│   │   └── sign_transaction.rs
│   ├── services/       # Core logic
│   │   ├── frost.rs
│   │   └── storage.rs
│   └── p2p/            # libp2p networking
│       ├── network.rs
│       └── protocol.rs
└── dist/               # Built extension
```

## Testing

### 1. Load Extension

Extension opens in right side panel. Check browser console:
```
✅ libp2p initialized! Peer ID: 12D3KooW...
```

### 2. Create Wallet (Current)

- Click "Create New Wallet"
- Choose "I'll Coordinate"
- Peer ID displays
- Connection/broadcast stubbed (Phase 2)

### 3. Next: Real P2P Testing

Once WebSocket transport implemented:
- Open two browser tabs
- Connect peers via libp2p
- Test message broadcast
- Run DKG coordination

## Development

### Watch Mode

```bash
cargo install cargo-watch
cargo watch -s './build.sh'
```

After rebuild, reload extension in `chrome://extensions`.

### Debug

Press F12 in side panel to see console logs.

## Implementation Plan

**See [MILESTONE.md](MILESTONE.md) for complete roadmap.**

### Immediate Focus: Milestone 1 (DKG)

**Goal:** Run FROST DKG across multiple browser tabs.

**Files to implement:**
- `src/services/frost.rs` - Complete DKG rounds
- `src/p2p/network.rs` - Gossipsub integration
- `src/p2p/protocol.rs` - Message types
- `src/components/create_wallet.rs` - DKG coordinator
- `src/services/storage.rs` - Wallet persistence

**Timeline:** 2-3 weeks

**Success:** Open 3 tabs → Run DKG → Each stores KeyPackage → All show same wallet address

## Resources

- [Dioxus](https://dioxuslabs.com)
- [libp2p](https://docs.rs/libp2p)
- [FROST](https://docs.rs/frost-secp256k1-tr)
- [Chrome Extensions](https://developer.chrome.com/docs/extensions/)

## License

MIT or Apache 2.0 (TBD)
