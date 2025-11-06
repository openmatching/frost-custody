// FROST Wallet - libp2p Circuit Relay v2 Server
//
// Provides relay service for browser nodes to:
// 1. Exchange signaling for WebRTC connections
// 2. Relay traffic when direct P2P not possible
//
// Deploy at: relay.frost-wallet.io

use futures::StreamExt;
use libp2p::{
    dcutr, gossipsub, identify, identity, noise, ping, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    yamux, Multiaddr, PeerId, SwarmBuilder,
};
use std::error::Error;

/// Relay server: Initial mesh via gossipsub, DCUtR upgrades to direct P2P
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "RelayEvent")]
struct RelayBehaviour {
    relay: relay::Behaviour,
    dcutr: dcutr::Behaviour,
    gossipsub: gossipsub::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

#[derive(Debug)]
enum RelayEvent {
    Relay(relay::Event),
    Dcutr(dcutr::Event),
    Gossipsub(gossipsub::Event),
    Ping(ping::Event),
    Identify(identify::Event),
}

impl From<relay::Event> for RelayEvent {
    fn from(event: relay::Event) -> Self {
        RelayEvent::Relay(event)
    }
}

impl From<dcutr::Event> for RelayEvent {
    fn from(event: dcutr::Event) -> Self {
        RelayEvent::Dcutr(event)
    }
}

impl From<gossipsub::Event> for RelayEvent {
    fn from(event: gossipsub::Event) -> Self {
        RelayEvent::Gossipsub(event)
    }
}

impl From<ping::Event> for RelayEvent {
    fn from(event: ping::Event) -> Self {
        RelayEvent::Ping(event)
    }
}

impl From<identify::Event> for RelayEvent {
    fn from(event: identify::Event) -> Self {
        RelayEvent::Identify(event)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("🚀 Starting FROST Relay Server");

    // Load or generate persistent keypair
    let keypair_path =
        std::env::var("RELAY_KEYPAIR_PATH").unwrap_or_else(|_| "./relay-keypair.bin".to_string());

    let local_key = load_or_generate_keypair(&keypair_path)?;
    let local_peer_id = PeerId::from(local_key.public());

    tracing::info!("📍 Peer ID: {} (persistent)", local_peer_id);

    // Build swarm with WebSocket (common, works everywhere)
    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            Default::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_websocket(noise::Config::new, yamux::Config::default)
        .await?
        .with_behaviour(|key| {
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(std::time::Duration::from_secs(10))
                .validation_mode(gossipsub::ValidationMode::Permissive)
                .mesh_outbound_min(1)
                .mesh_n_low(1)
                .mesh_n(2)
                .mesh_n_high(3)
                .build()
                .expect("Valid gossipsub config");

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )
            .expect("Valid gossipsub behaviour");

            // Configure relay with explicit limits
            let relay_config = relay::Config {
                max_reservations: 1024,
                max_reservations_per_peer: 16,
                reservation_duration: std::time::Duration::from_secs(3600), // 1 hour
                max_circuits: 16,
                max_circuits_per_peer: 8,
                ..Default::default()
            };

            Ok(RelayBehaviour {
                relay: relay::Behaviour::new(local_peer_id, relay_config),
                dcutr: dcutr::Behaviour::new(local_peer_id),
                gossipsub,
                ping: ping::Behaviour::new(ping::Config::new()),
                identify: identify::Behaviour::new(identify::Config::new(
                    "/frost-relay/1.0.0".to_string(),
                    key.public(),
                )),
            })
        })?
        .build();

    // Subscribe to frost topic for initial mesh formation
    let frost_topic = gossipsub::IdentTopic::new("/frost/room/my-wallet");
    swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(&frost_topic)
        .expect("Failed to subscribe");

    tracing::info!("🔧 Relay: WebSocket for signaling, browsers use WebRTC for P2P");

    // Listen on WebSocket (browsers connect here for signaling only)
    swarm.listen_on("/ip4/0.0.0.0/tcp/9091/ws".parse()?)?;
    swarm.listen_on("/ip6/::0/tcp/9091/ws".parse()?)?;

    tracing::info!("✅ Relay server ready!");
    tracing::info!("📊 Max reservations: 1024");
    tracing::info!("📊 Max circuits: 16");

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔗 Relay serves as signaling server only - NOT in data path!");
    println!(
        "   Address: /ip6/YOUR_IPV6/tcp/9091/ws/p2p/{}",
        local_peer_id
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Event loop
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(event) => {
                handle_behaviour_event(event);
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                tracing::info!("📍 Listening on: {}", address);
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                tracing::info!("✅ Connection from: {} via {:?}", peer_id, endpoint);
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                tracing::info!("❌ Connection closed: {} ({:?})", peer_id, cause);
            }
            SwarmEvent::IncomingConnection { send_back_addr, .. } => {
                tracing::debug!("📥 Incoming connection from: {}", send_back_addr);
            }
            _ => {}
        }
    }
}

fn handle_behaviour_event(event: RelayEvent) {
    match event {
        RelayEvent::Relay(event) => match event {
            relay::Event::ReservationReqAccepted { src_peer_id, .. } => {
                tracing::info!("✅ Reservation request ACCEPTED from {}", src_peer_id);
            }
            relay::Event::ReservationReqDenied { src_peer_id, .. } => {
                tracing::warn!("❌ Reservation request DENIED from {}", src_peer_id);
            }
            relay::Event::ReservationTimedOut { src_peer_id, .. } => {
                tracing::info!("⏱️ Reservation timed out for {}", src_peer_id);
            }
            relay::Event::CircuitReqDenied {
                src_peer_id,
                dst_peer_id,
                ..
            } => {
                tracing::warn!(
                    "❌ Circuit request DENIED: {} → {}",
                    src_peer_id,
                    dst_peer_id
                );
            }
            relay::Event::CircuitReqAccepted {
                src_peer_id,
                dst_peer_id,
            } => {
                tracing::info!("✅ Circuit ACCEPTED: {} ←→ {}", src_peer_id, dst_peer_id);
            }
            relay::Event::CircuitClosed {
                src_peer_id,
                dst_peer_id,
                ..
            } => {
                tracing::info!("🔌 Circuit closed: {} ←→ {}", src_peer_id, dst_peer_id);
            }
            _ => {
                tracing::debug!("🔗 Relay event: {:?}", event);
            }
        },
        RelayEvent::Dcutr(event) => {
            tracing::info!("🎯 DCUtR event: {:?}", event);
        }
        RelayEvent::Gossipsub(gossipsub::Event::Message {
            propagation_source,
            message,
            ..
        }) => {
            tracing::info!(
                "📨 Gossip msg from {}: {} bytes",
                propagation_source,
                message.data.len()
            );
        }
        RelayEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic }) => {
            tracing::info!("✅ {} subscribed to {}", peer_id, topic);
        }
        RelayEvent::Gossipsub(_) => {
            // Other gossipsub events (unsubscribed, etc.)
        }
        RelayEvent::Ping(event) => {
            tracing::debug!("🏓 Ping: {:?}", event);
        }
        RelayEvent::Identify(event) => {
            tracing::debug!("🆔 Identify: {:?}", event);
        }
    }
}

/// Load keypair from file or generate new one
fn load_or_generate_keypair(path: &str) -> Result<identity::Keypair, Box<dyn Error>> {
    if std::path::Path::new(path).exists() {
        tracing::info!("🔑 Loading keypair from: {}", path);

        let bytes = std::fs::read(path)?;
        let keypair = identity::Keypair::from_protobuf_encoding(&bytes)?;

        tracing::info!("✅ Keypair loaded (Peer ID will be consistent)");
        Ok(keypair)
    } else {
        tracing::info!("🔑 Generating new keypair...");

        let keypair = identity::Keypair::generate_ed25519();

        // Save for next time
        let bytes = keypair.to_protobuf_encoding()?;
        std::fs::write(path, &bytes)?;

        tracing::info!("✅ Keypair saved to: {}", path);
        tracing::info!("🔒 Peer ID will remain consistent across restarts");

        Ok(keypair)
    }
}
