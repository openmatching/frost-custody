// FROST protocol messages over libp2p
// Defines message types for DKG and signing

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FrostMessage {
    // Peer discovery
    PeerInfo {
        device_name: String,
        role: String,  // "coordinator" or "follower"
    },
    
    // DKG messages
    DkgRound1 {
        from: String,
        commitments: String,
    },
    DkgRound2 {
        from: String,
        shares: String,
    },
    
    // Signing messages
    SigningCommitments {
        from: String,
        commitments: String,
        message_hash: String,
    },
    SignatureShare {
        from: String,
        share: String,
    },
    
    // Coordination
    PeerList {
        peers: Vec<PeerEntry>,
    },
    StartDkg,
    StartSigning {
        message: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeerEntry {
    pub peer_id: String,
    pub device_name: String,
    pub role: String,
}

