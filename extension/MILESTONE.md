# Development Milestones

## Current Status: Phase 1 Complete ✅

**Foundation built:**
- Chrome extension structure ✅
- libp2p compiles in WASM ✅
- Peer ID generation ✅
- Pure Rust stack (Dioxus + libp2p + FROST) ✅
- No JavaScript dependencies ✅

## Milestone 1: DKG Working (Target: 2-3 weeks)

**Goal:** Create a wallet by running FROST DKG across multiple browser tabs.

### Task 1.1: FROST DKG Implementation

**File: `src/services/frost.rs`**

```rust
// Complete DKG coordinator
impl FrostCoordinator {
    // Round 1: Collect packages from all participants
    pub fn dkg_round1(
        &mut self,
        participants: &[Identifier],
    ) -> Result<Round1Package>;
    
    // Round 2: Process received packages
    pub fn dkg_round2(
        &mut self,
        round1_packages: Vec<Round1Package>,
    ) -> Result<(KeyPackage, PublicKeyPackage)>;
    
    // Helper: Serialize/deserialize packages
    pub fn serialize_package(pkg: &Round1Package) -> Vec<u8>;
    pub fn deserialize_package(data: &[u8]) -> Result<Round1Package>;
}
```

**Test:** Run DKG in single process (no P2P yet).

### Task 1.2: P2P Message Protocol

**File: `src/p2p/protocol.rs`**

```rust
#[derive(Serialize, Deserialize)]
pub enum FrostMessage {
    // DKG messages
    DkgRound1 {
        sender: u16,
        package: Vec<u8>,
    },
    DkgRound2 {
        sender: u16,
        package: Vec<u8>,
    },
    
    // Coordination
    Presence {
        device_name: String,
    },
    Ready {
        participants: Vec<u16>,
    },
}
```

**Test:** Serialize/deserialize messages successfully.

### Task 1.3: Gossipsub Integration

**File: `src/p2p/network.rs`**

**Architecture: Manual address sharing (maximum decentralization!)**

```rust
impl NetworkManager {
    // Coordinator: Start swarm, return address to share
    pub async fn init_coordinator(&mut self, room_id: &str) -> Result<String>;
    
    // Follower: Dial coordinator address, join room
    pub async fn init_follower(&mut self, room_id: &str, addr: &str) -> Result<()>;
    
    // Publish message to all subscribers
    pub fn broadcast(&mut self, msg: &[u8]) -> Result<()>;
    
    // Poll for incoming messages
    pub fn poll_messages(&mut self) -> Vec<(PeerId, Vec<u8>)>;
}
```

**Key insight:** Followers connect to coordinator, gossipsub forms mesh automatically!

**NOTE:** Skip authentication for MVP (whitelist in Milestone 2).

### Task 1.4: DKG Coordinator UI

**File: `src/components/create_wallet.rs`**

```rust
// Update StepConnect to run real DKG
async fn run_dkg_coordinator(
    peer_ctx: PeerContext,
    threshold: u16,
    total: u16,
) -> Result<Wallet> {
    // 1. Wait for all participants
    wait_for_peers(total).await?;
    
    // 2. Broadcast "start DKG"
    broadcast_start_dkg().await?;
    
    // 3. Run Round 1
    let my_pkg = frost.dkg_round1()?;
    broadcast_message(FrostMessage::DkgRound1 { my_pkg }).await?;
    
    // 4. Collect Round 1 packages
    let packages = collect_round1_packages(total).await?;
    
    // 5. Run Round 2
    let (key_pkg, group_key) = frost.dkg_round2(packages)?;
    
    // 6. Store wallet
    storage.save_wallet(key_pkg, group_key)?;
    
    Ok(wallet)
}
```

### Task 1.5: Storage

**File: `src/services/storage.rs`**

```rust
impl StorageManager {
    pub fn save_wallet(
        &self,
        key_package: &KeyPackage,
        group_public_key: &PublicKey,
    ) -> Result<()>;
    
    pub fn load_wallet(&self) -> Result<Option<Wallet>>;
}
```

### Success Criteria

**Can run DKG across 2-3 browser tabs:**
1. Open 3 tabs with extension
2. Tab 1: "Create Wallet" → Coordinator
3. Tab 2-3: "Join Wallet" → Followers
4. Enter room ID to connect
5. DKG runs automatically
6. All tabs show: "Wallet created! Address: 0x..."
7. Each tab has unique KeyPackage stored
8. All tabs have same group public key

**What's NOT in Milestone 1:**
- ❌ Message authentication (whitelist)
- ❌ Reconnection after disconnect
- ❌ Signing transactions
- ❌ Custody nodes
- ❌ BIP39 backup
- ❌ WebAuthn encryption

## Milestone 2: Message Authentication (Target: 1 week)

**Goal:** Secure P2P communication with whitelist.

### Task 2.1: Message Authentication Layer

**File: `src/p2p/message_auth.rs`**

Implement:
- Ed25519 keypair generation
- Message signing
- Signature verification
- Whitelist management

### Task 2.2: DKG Public Key Exchange

Extend DKG to exchange Ed25519 public keys for message authentication.

### Task 2.3: Authenticated Message Protocol

Wrap all messages in `AuthenticatedMessage` format with signature verification.

### Success Criteria

- Only whitelisted co-signers can participate
- Fake messages rejected automatically
- Replay attacks prevented

## Milestone 3: Transaction Signing (Target: 1-2 weeks)

**Goal:** Sign Ethereum transactions using FROST.

### Task 3.1: FROST Signing Implementation

Complete signing rounds in `src/services/frost.rs`.

### Task 3.2: Ethereum Integration

- Build unsigned transactions
- Compute signature hash
- Apply FROST signature
- Broadcast to RPC

### Task 3.3: Signing Coordinator UI

Update `src/components/sign_transaction.rs` for real signing flow.

### Success Criteria

- Send real ETH on testnet
- Transaction confirmed on-chain
- Signed with FROST threshold signature

## Milestone 4: Reconnection & Persistence (Target: 1 week)

**Goal:** Co-signers can reconnect after closing browser.

### Task 4.1: Wallet Identity Storage

Store `wallet_id`, `gossipsub_topic`, `cosigner_whitelist`.

### Task 4.2: Reconnection Logic

Implement hybrid discovery (gossipsub + relay).

### Task 4.3: Persistent Subscriptions

Auto-subscribe to wallet topics on extension startup.

### Success Criteria

- Close and reopen extension
- Automatically reconnect to co-signers
- Can sign transactions without re-running DKG

## Milestone 5: Custody Node MVP (Target: 2-3 weeks)

**Goal:** Always-online custody node with policy-based signing.

### Task 5.1: Custody Node Server

Rust server with libp2p + REST API.

### Task 5.2: Policy Engine

Implement `SigningPolicy` evaluation.

### Task 5.3: Subscription Management

User registration, payment, wallet management.

### Success Criteria

- User adds custody node to wallet
- Custody node auto-signs small transactions
- Requires manual approval for large amounts

## Milestone 6: Security & UX (Target: 2-3 weeks)

**Goal:** Production-ready security and user experience.

### Task 6.1: BIP39 Backup

- Generate mnemonic from KeyPackage
- Force user to backup
- Recovery flow

### Task 6.2: WebAuthn Encryption

- Encrypt KeyPackage with hardware key
- Touch ID / Face ID unlock
- Memory zeroing

### Task 6.3: Error Handling

- Network failures
- Insufficient signers
- Invalid messages
- User-friendly errors

### Task 6.4: UI Polish

- Loading states
- Progress indicators
- Success animations
- Help tooltips

### Success Criteria

- KeyPackages encrypted at rest
- Can recover from mnemonic
- Clean, professional UI
- No crashes or confusing errors

## Total Timeline

**Conservative estimate: 8-12 weeks**

- Week 1-3: Milestone 1 (DKG) ← **START HERE**
- Week 4: Milestone 2 (Auth)
- Week 5-6: Milestone 3 (Signing)
- Week 7: Milestone 4 (Reconnection)
- Week 8-10: Milestone 5 (Custody)
- Week 11-12: Milestone 6 (Polish)

**Current focus:** Get DKG working across multiple tabs. Everything else builds on this foundation.

