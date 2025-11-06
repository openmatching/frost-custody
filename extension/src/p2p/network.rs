// libp2p Network Manager for WASM
// Handles P2P connections via WebSocket + Gossipsub (via relay)

use futures::{FutureExt, StreamExt};
use libp2p::{
    core::upgrade, gossipsub, identity, noise, swarm, websocket_websys, yamux, Multiaddr, PeerId,
    Swarm, SwarmBuilder, Transport,
};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures;
use web_sys::{
    MessageEvent, RtcConfiguration, RtcDataChannel, RtcIceServer, RtcPeerConnection, RtcSdpType,
    RtcSessionDescriptionInit,
};

/// Network manager for P2P communication
///
/// Architecture:
/// - WebSocket transport to relay server
/// - Gossipsub for P2P messaging (relay forwards messages)
/// - Messages sent via channel to event loop for publishing
///
/// Flow:
/// 1. Both nodes connect to relay via WebSocket
/// 2. Subscribe to gossipsub topic
/// 3. Messages sent through channel, published by event loop
/// 4. Gossipsub mesh routes messages peer-to-peer via relay
#[derive(Clone)]
pub struct NetworkManager {
    inner: std::rc::Rc<NetworkManagerInner>,
}

struct NetworkManagerInner {
    keypair: identity::Keypair,
    peer_id: PeerId,
    room_id: RefCell<Option<String>>,

    // libp2p swarm for relay connection
    swarm: RefCell<Option<Swarm<FrostBehaviour>>>,
    relay_connected: RefCell<bool>,
    connected_peers: RefCell<std::collections::HashSet<PeerId>>,

    // Track gossipsub topic subscribers (actual participants, not just relay)
    topic_peers: RefCell<std::collections::HashSet<PeerId>>,

    // WebRTC connection (direct P2P) - not used currently, for future upgrade
    rtc_connection: RefCell<Option<RtcPeerConnection>>,
    data_channel: RefCell<Option<RtcDataChannel>>,

    // Message channels for publish and receive (fully reactive)
    message_tx: RefCell<Option<futures::channel::mpsc::UnboundedSender<Vec<u8>>>>,
    message_rx: RefCell<Option<futures::channel::mpsc::UnboundedReceiver<Vec<u8>>>>,
    received_message_tx:
        RefCell<Option<futures::channel::mpsc::UnboundedSender<(String, Vec<u8>)>>>,
}

/// libp2p behaviour for FROST - WebSocket + Gossipsub
///
/// Architecture: Pure WebSocket transport with Gossipsub for P2P messaging
/// Upgrade Path: When rust-libp2p PR #5978 merges, swap websocket_websys for webrtc_websys
#[derive(libp2p::swarm::NetworkBehaviour)]
#[behaviour(to_swarm = "FrostEvent")]
pub struct FrostBehaviour {
    gossipsub: gossipsub::Behaviour,
    ping: libp2p::ping::Behaviour,
    identify: libp2p::identify::Behaviour,
}

#[derive(Debug)]
pub enum FrostEvent {
    Gossipsub(gossipsub::Event),
    Ping(libp2p::ping::Event),
    Identify(libp2p::identify::Event),
}

impl From<gossipsub::Event> for FrostEvent {
    fn from(event: gossipsub::Event) -> Self {
        FrostEvent::Gossipsub(event)
    }
}

impl From<libp2p::ping::Event> for FrostEvent {
    fn from(event: libp2p::ping::Event) -> Self {
        FrostEvent::Ping(event)
    }
}

impl From<libp2p::identify::Event> for FrostEvent {
    fn from(event: libp2p::identify::Event) -> Self {
        FrostEvent::Identify(event)
    }
}

// Relay server address (WebSocket)
const RELAY_ADDR: &str = "/ip6/2406:da1e:245:2400:1f0:a3de:27c4:9e21/tcp/9091/ws/p2p/12D3KooWJhk3JLp4gVZVJAVYWMUxgmNckZCAycvjaSyQQ3aGhwv9";
const RELAY_PEER_ID: &str = "12D3KooWJhk3JLp4gVZVJAVYWMUxgmNckZCAycvjaSyQQ3aGhwv9";
const STUN_SERVER: &str = "stun:stun.l.google.com:19302";

impl NetworkManager {
    /// Create new network manager with message channels pre-initialized
    pub fn new() -> Self {
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());

        // Create message channels immediately so broadcast() always works
        let (message_tx, message_rx) = futures::channel::mpsc::unbounded::<Vec<u8>>();
        let (received_tx, _received_rx) = futures::channel::mpsc::unbounded::<(String, Vec<u8>)>();

        Self {
            inner: std::rc::Rc::new(NetworkManagerInner {
                keypair,
                peer_id,
                room_id: RefCell::new(None),
                swarm: RefCell::new(None),
                relay_connected: RefCell::new(false),
                connected_peers: RefCell::new(std::collections::HashSet::new()),
                topic_peers: RefCell::new(std::collections::HashSet::new()),
                rtc_connection: RefCell::new(None),
                data_channel: RefCell::new(None),
                message_tx: RefCell::new(Some(message_tx)),
                message_rx: RefCell::new(Some(message_rx)),
                received_message_tx: RefCell::new(Some(received_tx)),
            }),
        }
    }

    /// Build libp2p swarm with WebSocket transport for WASM
    ///
    /// Current: WebSocket transport via relay server
    /// Future Upgrade: Replace websocket_websys with webrtc_websys when PR #5978 merges
    async fn build_swarm(&self) -> Result<Swarm<FrostBehaviour>, JsValue> {
        // Configure gossipsub with relaxed settings for small peer counts
        // Must satisfy: mesh_outbound_min <= mesh_n_low <= mesh_n <= mesh_n_high
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Permissive) // Allow publishing with few peers
            .mesh_outbound_min(1) // Min outbound connections (default 2)
            .mesh_n_low(1) // Min peers in mesh (default 4)
            .mesh_n(2) // Target peers in mesh (default 6)
            .mesh_n_high(3) // Max peers in mesh (default 12)
            .build()
            .map_err(|e| JsValue::from_str(&format!("Gossipsub config error: {}", e)))?;

        let keypair_clone = self.inner.keypair.clone();

        // Setup WebSocket transport
        // UPGRADE PATH: When rust-libp2p PR #5978 merges, replace this with:
        // webrtc_websys::Transport::new(...) for direct browser-to-browser P2P
        let swarm = SwarmBuilder::with_existing_identity(keypair_clone)
            .with_wasm_bindgen()
            .with_other_transport(|key| {
                // WebSocket transport - connects via relay server
                websocket_websys::Transport::default()
                    .upgrade(upgrade::Version::V1)
                    .authenticate(noise::Config::new(&key).unwrap())
                    .multiplex(yamux::Config::default())
            })
            .map_err(|e| JsValue::from_str(&format!("Transport error: {:?}", e)))?
            .with_behaviour(|key| {
                // Create gossipsub for P2P messaging over relay
                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )
                .map_err(|e| format!("Gossipsub error: {}", e))?;

                Ok(FrostBehaviour {
                    gossipsub,
                    ping: libp2p::ping::Behaviour::new(libp2p::ping::Config::new()),
                    identify: libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
                        "/frost/1.0.0".to_string(),
                        key.public(),
                    )),
                })
            })
            .map_err(|e| JsValue::from_str(&format!("Behaviour error: {:?}", e)))?
            .build();

        Ok(swarm)
    }

    /// Connect to relay server
    async fn connect_to_relay(&mut self) -> Result<(), JsValue> {
        log::info!("🔗 Connecting to relay: {}", RELAY_ADDR);

        // Build swarm if not exists
        if self.inner.swarm.borrow().is_none() {
            *self.inner.swarm.borrow_mut() = Some(self.build_swarm().await?);
        }

        // Dial relay
        let relay_addr: Multiaddr = RELAY_ADDR
            .parse()
            .map_err(|e| JsValue::from_str(&format!("Invalid relay addr: {}", e)))?;

        if let Some(swarm) = &mut *self.inner.swarm.borrow_mut() {
            swarm
                .dial(relay_addr)
                .map_err(|e| JsValue::from_str(&format!("Dial error: {:?}", e)))?;
        }

        // Wait for connection (poll swarm events)
        // TODO: Implement proper event loop

        log::info!("✅ Connected to relay");
        *self.inner.relay_connected.borrow_mut() = true;

        Ok(())
    }

    /// Create WebRTC peer connection with STUN
    fn create_rtc_peer_connection(&self) -> Result<RtcPeerConnection, JsValue> {
        // Configure STUN servers
        let ice_server = RtcIceServer::new();
        ice_server.set_urls(&JsValue::from_str(STUN_SERVER));

        let config = RtcConfiguration::new();
        config.set_ice_servers(&js_sys::Array::of1(&ice_server));

        // Create peer connection
        RtcPeerConnection::new_with_configuration(&config)
    }

    /// Initialize as coordinator
    /// Returns: Relayed multiaddr that followers should dial
    pub async fn init_coordinator(&mut self, room_id: &str) -> Result<String, JsValue> {
        log::info!("🎯 Initializing as coordinator for room: {}", room_id);
        *self.inner.room_id.borrow_mut() = Some(room_id.to_string());

        // Step 1: Connect to relay server
        self.connect_to_relay().await?;

        // Step 2: Subscribe to gossipsub topic for room
        let topic = gossipsub::IdentTopic::new(format!("/frost/room/{}", room_id));

        if let Some(swarm) = &mut *self.inner.swarm.borrow_mut() {
            swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&topic)
                .map_err(|e| JsValue::from_str(&format!("Subscribe error: {}", e)))?;

            log::info!("✅ Subscribed to topic: /frost/room/{}", room_id);

            // Step 3: Return coordinator's peer ID for followers to dial
            // Followers will connect to relay, then gossipsub mesh will handle P2P messaging
            //
            // UPGRADE PATH: When WebRTC b2b is available, this will become:
            // format!("{}/p2p-circuit/p2p/{}/p2p-circuit/webrtc/p2p/{}", RELAY_ADDR, peer_id, peer_id)

            let coordinator_peer_id = self.inner.peer_id.to_string();

            log::info!("📍 Coordinator Peer ID: {}", coordinator_peer_id);
            log::info!("✅ Coordinator ready! Followers will connect via gossipsub");
            log::info!("💡 Both peers connect to relay, gossipsub mesh handles P2P messaging");

            // Return just the peer ID - follower will use it to filter gossipsub messages
            Ok(coordinator_peer_id)
        } else {
            Err(JsValue::from_str("Swarm not initialized"))
        }
    }

    /// Initialize as follower
    /// Connects to relay and joins gossipsub mesh for P2P messaging
    pub async fn init_follower(&mut self, coordinator_peer_id: &str) -> Result<(), JsValue> {
        log::info!("🎯 Initializing as follower");
        log::info!("📍 Coordinator Peer ID: {}", coordinator_peer_id);

        let room_id = "my-wallet";
        *self.inner.room_id.borrow_mut() = Some(room_id.to_string());

        // Step 1: Connect to relay server
        self.connect_to_relay().await?;

        // Step 2: Subscribe to gossipsub topic (for DKG/signing messages)
        // Both coordinator and follower connect to relay, gossipsub mesh handles P2P
        let topic = gossipsub::IdentTopic::new(format!("/frost/room/{}", room_id));

        if let Some(swarm) = &mut *self.inner.swarm.borrow_mut() {
            swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&topic)
                .map_err(|e| JsValue::from_str(&format!("Subscribe error: {}", e)))?;

            log::info!("✅ Subscribed to topic: /frost/room/{}", room_id);
            log::info!("✅ Follower ready! Connected to gossipsub mesh via relay");
            log::info!("💡 Messages will flow: Follower <-> Relay <-> Coordinator");
            log::info!("🔮 Future: With WebRTC b2b, it will be: Follower <-> Coordinator (direct)");

            // Note: No need to dial coordinator directly - gossipsub mesh handles routing
            // The relay server forwards gossipsub messages between peers

            Ok(())
        } else {
            Err(JsValue::from_str("Swarm not initialized"))
        }
    }

    /// Setup data channel event handlers
    fn setup_data_channel_handlers(&self, channel: &RtcDataChannel) -> Result<(), JsValue> {
        // On open
        let onopen_callback = Closure::wrap(Box::new(move || {
            log::info!("✅ WebRTC data channel opened!");
        }) as Box<dyn FnMut()>);
        channel.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        // On message
        let onmessage_callback = Closure::wrap(Box::new(move |ev: MessageEvent| {
            if let Ok(txt) = ev.data().dyn_into::<js_sys::JsString>() {
                log::info!("📨 Received message: {}", String::from(txt));
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        channel.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // On error
        let onerror_callback = Closure::wrap(Box::new(move |_ev: JsValue| {
            log::error!("❌ Data channel error");
        }) as Box<dyn FnMut(JsValue)>);
        channel.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        Ok(())
    }

    /// Create WebRTC offer (coordinator)
    pub async fn create_offer(&mut self) -> Result<String, JsValue> {
        let rtc = self
            .inner
            .rtc_connection
            .borrow()
            .clone()
            .ok_or_else(|| JsValue::from_str("No RTC connection"))?;

        let offer = wasm_bindgen_futures::JsFuture::from(rtc.create_offer()).await?;
        let offer_sdp = js_sys::Reflect::get(&offer, &JsValue::from_str("sdp"))?
            .as_string()
            .ok_or_else(|| JsValue::from_str("No SDP in offer"))?;

        // Set local description
        let offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_obj.set_sdp(&offer_sdp);
        let set_local = rtc.set_local_description(&offer_obj);
        wasm_bindgen_futures::JsFuture::from(set_local).await?;

        log::info!("✅ Created WebRTC offer");
        Ok(offer_sdp)
    }

    /// Handle WebRTC answer (coordinator)
    pub async fn handle_answer(&mut self, answer_sdp: &str) -> Result<(), JsValue> {
        let rtc = self
            .inner
            .rtc_connection
            .borrow()
            .clone()
            .ok_or_else(|| JsValue::from_str("No RTC connection"))?;

        let answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.set_sdp(answer_sdp);

        let set_remote = rtc.set_remote_description(&answer_obj);
        wasm_bindgen_futures::JsFuture::from(set_remote).await?;

        log::info!("✅ Set remote answer");
        Ok(())
    }

    /// Create WebRTC answer (follower)
    pub async fn create_answer(&mut self, offer_sdp: &str) -> Result<String, JsValue> {
        let rtc = self
            .inner
            .rtc_connection
            .borrow()
            .as_ref()
            .ok_or_else(|| JsValue::from_str("No RTC connection"))?
            .clone();

        // Set remote description (offer from coordinator)
        let offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_obj.set_sdp(offer_sdp);

        let set_remote = rtc.set_remote_description(&offer_obj);
        wasm_bindgen_futures::JsFuture::from(set_remote).await?;

        // Create answer
        let answer = wasm_bindgen_futures::JsFuture::from(rtc.create_answer()).await?;
        let answer_sdp = js_sys::Reflect::get(&answer, &JsValue::from_str("sdp"))?
            .as_string()
            .ok_or_else(|| JsValue::from_str("No SDP in answer"))?;

        // Set local description
        let answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.set_sdp(&answer_sdp);
        let set_local = rtc.set_local_description(&answer_obj);
        wasm_bindgen_futures::JsFuture::from(set_local).await?;

        log::info!("✅ Created WebRTC answer");
        Ok(answer_sdp)
    }

    /// Broadcast message to all peers via gossipsub
    ///
    /// Messages are sent via channel to event loop for immediate publishing
    pub fn broadcast(&mut self, message: &[u8]) -> Result<(), JsValue> {
        let _room_id = self
            .inner
            .room_id
            .borrow()
            .clone()
            .ok_or_else(|| JsValue::from_str("No room ID"))?;

        // Send message through channel - event loop will publish immediately
        if let Some(tx) = &*self.inner.message_tx.borrow() {
            tx.unbounded_send(message.to_vec())
                .map_err(|e| JsValue::from_str(&format!("Channel send error: {}", e)))?;
            log::info!(
                "📤 Sent message ({} bytes) to publish channel",
                message.len()
            );
        } else {
            log::error!("❌ Message channel not initialized!");
            return Err(JsValue::from_str("Message channel not initialized"));
        }

        Ok(())
    }

    /// Start background event loop to poll swarm and publish messages
    /// Must be called after init_coordinator() or init_follower()
    ///
    /// Returns: Receiver for incoming messages (use this for reactive UI updates)
    pub fn start_event_loop(&self) -> futures::channel::mpsc::UnboundedReceiver<(String, Vec<u8>)> {
        let this = self.clone();

        // Take the message receiver that was created in new()
        let mut message_rx = self
            .inner
            .message_rx
            .borrow_mut()
            .take()
            .expect("Message receiver already consumed! Only call start_event_loop() once");

        // Create receiver for UI (returned)
        let (received_tx, received_rx) = futures::channel::mpsc::unbounded::<(String, Vec<u8>)>();
        *this.inner.received_message_tx.borrow_mut() = Some(received_tx.clone());

        wasm_bindgen_futures::spawn_local(async move {
            let swarm = &this.inner.swarm;
            let received_message_tx = &this.inner.received_message_tx;
            let connected_peers = &this.inner.connected_peers;
            let topic_peers = &this.inner.topic_peers;
            let room_id = &this.inner.room_id;

            log::info!("🔄 Starting swarm event loop with message channels...");

            loop {
                // Use select! to wait on BOTH swarm events AND outgoing messages
                futures::select! {
                    // Swarm events (connection, gossipsub, etc)
                    swarm_event = async {
                        let swarm_opt = &mut *swarm.borrow_mut();
                        if let Some(swarm_ref) = swarm_opt {
                            Some(swarm_ref.select_next_some().await)
                        } else {
                            None
                        }
                    }.fuse() => {
                        if let Some(ev) = swarm_event {
                            Self::handle_swarm_event_static(ev, received_message_tx, &connected_peers, &topic_peers);
                        } else {
                            // Swarm not initialized, wait a bit
                            gloo_timers::future::TimeoutFuture::new(100).await;
                        }
                    },

                    // Outgoing messages from broadcast()
                    message = message_rx.select_next_some() => {
                        log::info!("📨 Received message from channel ({} bytes)", message.len());

                        if let Some(room) = &*room_id.borrow() {
                            let topic = gossipsub::IdentTopic::new(format!("/frost/room/{}", room));

                            if let Ok(mut swarm_guard) = swarm.try_borrow_mut() {
                                if let Some(swarm_ref) = &mut *swarm_guard {
                                    // Check mesh status
                                    let mesh_peers: Vec<_> = swarm_ref.behaviour().gossipsub.mesh_peers(&topic.hash()).collect();
                                    log::info!("📊 Publishing to {} mesh peers", mesh_peers.len());

                                    match swarm_ref.behaviour_mut().gossipsub.publish(topic, message.clone()) {
                                        Ok(msg_id) => {
                                            log::info!("✅ Published to gossipsub (msg_id: {:?})", msg_id);
                                        }
                                        Err(e) => {
                                            log::error!("❌ Publish failed: {:?}", e);
                                        }
                                    }
                                } else {
                                    log::error!("❌ Swarm not initialized, dropping message");
                                }
                            } else {
                                log::error!("❌ Swarm still busy, dropping message");
                            }
                        }
                    },
                }
            }
        });

        // Return receiver for incoming messages
        received_rx
    }

    /// Handle swarm event (static version for use in event loop)
    fn handle_swarm_event_static(
        event: swarm::SwarmEvent<FrostEvent>,
        received_message_tx: &RefCell<
            Option<futures::channel::mpsc::UnboundedSender<(String, Vec<u8>)>>,
        >,
        connected_peers: &RefCell<std::collections::HashSet<PeerId>>,
        topic_peers: &RefCell<std::collections::HashSet<PeerId>>,
    ) {
        match event {
            swarm::SwarmEvent::Behaviour(FrostEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source,
                message,
                ..
            })) => {
                log::info!(
                    "📨 Received gossipsub message from {:?}: {} bytes",
                    propagation_source,
                    message.data.len()
                );

                // Track the sender as a participant (they're clearly on the topic!)
                let sender_str = propagation_source.to_string();
                if !sender_str.contains(RELAY_PEER_ID) {
                    let was_new = topic_peers.borrow_mut().insert(propagation_source);
                    if was_new {
                        log::info!(
                            "👥 Discovered participant via message: {}",
                            propagation_source
                        );
                        log::info!("📊 Total participants: {}", topic_peers.borrow().len());
                    }
                }

                // Send through channel to UI
                if let Some(tx) = &*received_message_tx.borrow() {
                    let _ =
                        tx.unbounded_send((propagation_source.to_string(), message.data.clone()));
                }
            }
            swarm::SwarmEvent::Behaviour(FrostEvent::Identify(event)) => {
                log::info!("🆔 Identify event: {:?}", event);
            }
            swarm::SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                log::info!("✅ Peer connected: {} via {:?}", peer_id, endpoint);
                connected_peers.borrow_mut().insert(peer_id);
            }
            swarm::SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                log::info!("❌ Peer disconnected: {} (cause: {:?})", peer_id, cause);
                connected_peers.borrow_mut().remove(&peer_id);
                topic_peers.borrow_mut().remove(&peer_id);
            }
            swarm::SwarmEvent::NewListenAddr { address, .. } => {
                log::info!("🎧 New listen address: {}", address);
            }
            swarm::SwarmEvent::IncomingConnection { .. } => {
                log::info!("🔔 Incoming connection...");
            }
            swarm::SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                log::warn!("⚠️ Outgoing connection error to {:?}: {:?}", peer_id, error);
            }
            swarm::SwarmEvent::Behaviour(FrostEvent::Gossipsub(event)) => match event {
                gossipsub::Event::Subscribed { peer_id, topic } => {
                    log::info!("✅ Gossipsub peer {} subscribed to {}", peer_id, topic);

                    // Track participants (peers subscribed to our topic, excluding relay)
                    if topic.as_str().starts_with("/frost/room/") {
                        let peer_str = peer_id.to_string();
                        if !peer_str.contains(RELAY_PEER_ID) {
                            topic_peers.borrow_mut().insert(peer_id);
                            log::info!(
                                "👥 Browser participant joined! Total: {}",
                                topic_peers.borrow().len()
                            );
                        } else {
                            log::info!("🔗 Relay server subscribed (not counted as participant)");
                        }
                    }
                }
                gossipsub::Event::Unsubscribed { peer_id, topic } => {
                    log::warn!("❌ Gossipsub peer {} unsubscribed from {}", peer_id, topic);

                    // Remove from participants
                    if topic.as_str().starts_with("/frost/room/") {
                        topic_peers.borrow_mut().remove(&peer_id);
                        log::info!(
                            "👥 Participant left. Total participants: {}",
                            topic_peers.borrow().len()
                        );
                    }
                }
                gossipsub::Event::GossipsubNotSupported { peer_id } => {
                    log::warn!("⚠️ Peer {} doesn't support gossipsub", peer_id);
                }
                _ => {
                    log::debug!("🔍 Other gossipsub event: {:?}", event);
                }
            },
            other => {
                log::debug!("Unhandled swarm event: {:?}", other);
            }
        }
    }

    /// Get peer ID
    pub fn get_peer_id(&self) -> String {
        self.inner.peer_id.to_string()
    }

    /// Get connected peer count
    pub fn peer_count(&self) -> usize {
        self.inner.connected_peers.borrow().len()
    }

    /// Get list of all connected peer IDs (WebSocket connections)
    pub fn get_connected_peers(&self) -> Vec<String> {
        self.inner
            .connected_peers
            .borrow()
            .iter()
            .map(|peer_id| peer_id.to_string())
            .collect()
    }

    /// Get list of gossipsub topic participants (other browsers on same topic)
    /// This is what you want to display in the UI - actual FROST participants!
    /// Excludes the relay server from the list.
    pub fn get_participants(&self) -> Vec<String> {
        self.inner
            .topic_peers
            .borrow()
            .iter()
            .filter(|peer_id| {
                // Exclude relay server
                let peer_str = peer_id.to_string();
                !peer_str.contains(RELAY_PEER_ID)
            })
            .map(|peer_id| peer_id.to_string())
            .collect()
    }

    /// Get participant count (other browsers, excluding relay)
    pub fn participant_count(&self) -> usize {
        self.get_participants().len()
    }

    /// Get connection status
    pub fn is_connected(&self) -> bool {
        *self.inner.relay_connected.borrow() && self.peer_count() > 0
    }

    /// Set room ID (for broadcast when network is created separately)
    pub fn set_room_id(&mut self, room_id: String) {
        *self.inner.room_id.borrow_mut() = Some(room_id);
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}
