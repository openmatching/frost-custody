use crate::icons;
use crate::p2p::NetworkManager;
use crate::peer_context::{PeerContext, PeerInfo};
use crate::Role;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct PeerListMessage {
    msg_type: String,
    peers: Vec<PeerInfo>,
}

#[component]
pub fn CreateWallet(role: Role, on_complete: EventHandler<()>) -> Element {
    let mut step = use_signal(|| 1);
    let room_id = use_signal(|| String::new());
    let remote_peer_id = use_signal(|| String::new());
    let is_coordinator = role == Role::Coordinator;

    rsx! {
        div { class: "max-w-2xl mx-auto mt-10 p-8 bg-white rounded-lg shadow-lg",
            // Progress indicator
            div { class: "mb-8",
                div { class: "flex justify-between items-center",
                    for i in 1..=3 {
                        div { class: if step() >= i { "flex-1 border-t-4 border-blue-600" } else { "flex-1 border-t-4 border-gray-200" } }
                    }
                }
                div { class: "flex justify-between mt-2 text-sm",
                    span { class: if step() >= 1 { "text-blue-600" } else { "text-gray-400" }, "Connect" }
                    span { class: if step() >= 2 { "text-blue-600" } else { "text-gray-400" }, "DKG" }
                    span { class: if step() >= 3 { "text-blue-600" } else { "text-gray-400" }, "Complete" }
                }
            }

            match step() {
                1 => rsx! { StepConnect {
                    room_id,
                    remote_peer_id,
                    is_coordinator,
                    on_next: move |_| step.set(2)
                } },
                2 => rsx! { StepDKG {
                    is_coordinator,
                    on_next: move |_| step.set(3)
                } },
                3 => rsx! { StepComplete { on_complete } },
                _ => rsx! { div { "Unknown step" } }
            }
        }
    }
}

#[component]
fn StepConnect(
    room_id: Signal<String>,
    mut remote_peer_id: Signal<String>,
    is_coordinator: bool,
    on_next: EventHandler<()>,
) -> Element {
    let peer_ctx = use_context::<PeerContext>();
    let mut peer_count = use_signal(|| 0);
    let mut status_message = use_signal(|| String::from("Ready"));
    let mut messages = use_signal(|| Vec::<String>::new());
    let mut connected_peers = use_signal(|| Vec::<String>::new());

    let ctx_for_connect = peer_ctx.clone();
    let ctx_for_broadcast = peer_ctx.clone();

    // Initialize coordinator on component mount
    let ctx_for_init = peer_ctx.clone();
    use_effect(move || {
        if is_coordinator {
            let ctx = ctx_for_init.clone();
            let mut msgs = messages.clone();
            spawn(async move {
                log::info!("Initializing libp2p as coordinator...");

                let mut network = NetworkManager::new();
                let room = "my-wallet";

                ctx.set_room_id(room.to_string());

                log::info!("🔄 Starting coordinator initialization...");
                match network.init_coordinator(room).await {
                    Ok(addr) => {
                        log::info!("✅ Coordinator ready! Peer ID: {}", addr);
                        room_id.set(addr.clone());

                        // Start event loop and get message receiver
                        let mut rx = network.start_event_loop();

                        // Store network in context
                        ctx.set_network(network);

                        // Spawn task to listen to messages reactively
                        wasm_bindgen_futures::spawn_local(async move {
                            use futures::StreamExt;
                            while let Some((peer_id, msg_bytes)) = rx.next().await {
                                let msg_str = String::from_utf8_lossy(&msg_bytes).to_string();
                                log::info!("📨 UI: Received message from {}: {}", peer_id, msg_str);
                                msgs.write().push(format!(
                                    "📥 From {}: {}",
                                    &peer_id[..8],
                                    msg_str
                                ));
                            }
                        });

                        log::info!("✅ Coordinator initialization complete!");
                    }
                    Err(e) => {
                        log::error!("❌ Failed to init coordinator: {:?}", e);
                    }
                }
            });
        }
    });

    // Poll for participant list every 500ms (messages are reactive via channel)
    {
        let ctx_for_polling = peer_ctx.clone();
        let peers_for_polling = connected_peers.clone();
        let count_for_polling = peer_count.clone();
        use_future(move || {
            let ctx = ctx_for_polling.clone();
            let mut peers = peers_for_polling.clone();
            let mut count = count_for_polling.clone();
            async move {
                loop {
                    gloo_timers::future::TimeoutFuture::new(500).await;

                    if let Some(network) = ctx.get_network() {
                        // Poll for participants (gossipsub topic subscribers = other browsers)
                        let participant_list = network.get_participants();
                        if participant_list != *peers.read() {
                            log::info!(
                                "👥 Participants updated: {} browsers on topic",
                                participant_list.len()
                            );
                            peers.set(participant_list.clone());
                            count.set(participant_list.len());
                        }
                    }
                }
            }
        });
    }

    // Connect to coordinator via relay
    let handle_connect = move |_| {
        let coordinator_addr = remote_peer_id();
        if coordinator_addr.is_empty() {
            status_message.set("Please enter coordinator address".to_string());
            return;
        }

        let ctx = ctx_for_connect.clone();
        let mut msgs = messages.clone();

        spawn(async move {
            log::info!("🔗 Connecting to coordinator: {}", coordinator_addr);
            status_message.set(format!("Connecting to relay..."));

            let mut network = crate::p2p::network::NetworkManager::new();

            // Set room ID before initializing
            ctx.set_room_id("my-wallet".to_string());

            match network.init_follower(&coordinator_addr).await {
                Ok(_) => {
                    log::info!("✅ Connected to coordinator via relay");
                    status_message.set("Connected!".to_string());

                    // Start event loop and get message receiver
                    let mut rx = network.start_event_loop();

                    // Store network in context
                    ctx.set_network(network);

                    // Spawn task to listen to messages reactively
                    wasm_bindgen_futures::spawn_local(async move {
                        use futures::StreamExt;
                        while let Some((peer_id, msg_bytes)) = rx.next().await {
                            let msg_str = String::from_utf8_lossy(&msg_bytes).to_string();
                            log::info!("📨 UI: Received message from {}: {}", peer_id, msg_str);
                            msgs.write()
                                .push(format!("📥 From {}: {}", &peer_id[..8], msg_str));
                        }
                    });

                    peer_count.set(1);
                }
                Err(e) => {
                    log::error!("Failed to connect: {:?}", e);
                    status_message.set(format!("Connection failed: {:?}", e));
                }
            }
        });
    };

    // DEBUG: Broadcast hello message
    let ctx_for_hello = peer_ctx.clone();
    let handle_broadcast_hello = move |_: dioxus::prelude::Event<dioxus::html::MouseData>| {
        let ctx = ctx_for_hello.clone();
        spawn(async move {
            log::info!("🔊 Broadcasting hello message...");

            // Get NetworkManager from context
            let mut network = match ctx.get_network() {
                Some(net) => net,
                None => {
                    log::error!("NetworkManager not initialized!");
                    status_message.set("Error: Network not ready".to_string());
                    return;
                }
            };

            let hello_msg = format!("Hello from {}!", ctx.get_device_name());
            match network.broadcast(hello_msg.as_bytes()) {
                Ok(_) => {
                    log::info!("✅ Broadcast sent!");
                    status_message.set("Hello broadcasted!".to_string());
                    messages.write().push(format!("📤 Sent: {}", hello_msg));
                }
                Err(e) => {
                    log::error!("❌ Broadcast failed: {:?}", e);
                    status_message.set(format!("Broadcast failed: {:?}", e));
                }
            }
        });
    };

    // Broadcast peer list
    let handle_broadcast_peers = move |_| {
        if !is_coordinator {
            return;
        }

        let ctx = ctx_for_broadcast.clone();

        spawn(async move {
            let peers = ctx.get_peers();
            let message = PeerListMessage {
                msg_type: "peer_list".to_string(),
                peers,
            };

            let json = serde_json::to_string(&message).unwrap();
            log::info!("📡 Broadcasting peer list: {}", json);

            status_message.set("Peer list broadcast".to_string());
        });
    };

    rsx! {
        div { class: "space-y-6",
            h2 { class: "text-2xl font-bold text-gray-900", "Connect Devices" }

            p { class: "text-gray-600",
                "Share your Peer ID with other devices"
            }

            div { class: "bg-blue-50 border border-blue-200 rounded-lg p-3 mb-4",
                p { class: "text-sm text-blue-800 text-center",
                    "{status_message}"
                }
            }

            // Peer ID
            div { class: "bg-gradient-to-br from-blue-50 to-blue-100 border-2 border-blue-200 rounded-lg p-6",
                p { class: "text-sm font-medium text-blue-900 mb-2",
                    "Your Peer ID:"
                }
                div { class: "bg-white rounded-lg p-4 mb-4",
                    p { class: "text-lg font-mono text-center text-gray-900 select-all break-all",
                        if !room_id().is_empty() {
                            "{room_id}"
                        } else {
                            span { class: "text-gray-400", "Generating..." }
                        }
                    }
                }
                button {
                    class: "w-full bg-blue-600 text-white py-2 px-4 rounded-lg hover:bg-blue-700 transition text-sm flex items-center justify-center",
                    disabled: room_id().is_empty(),
                    icons::Copy { class: Some("w-4 h-4 mr-2".to_string()) }
                    "Copy Peer ID"
                }
            }

            // Connect
            div { class: "bg-gray-50 border border-gray-200 rounded-lg p-4",
                p { class: "text-sm font-medium text-gray-700 mb-3",
                    "Or connect to another device:"
                }
                div { class: "flex space-x-2",
                    input {
                        class: "flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm font-mono",
                        r#type: "text",
                        placeholder: "frost-abc123...",
                        value: "{remote_peer_id}",
                        oninput: move |e| remote_peer_id.set(e.value().clone())
                    }
                    button {
                        class: "bg-green-600 text-white px-4 py-2 rounded-lg hover:bg-green-700 transition text-sm",
                        disabled: remote_peer_id().is_empty(),
                        onclick: handle_connect,
                        "Connect"
                    }
                }
            }

            // Peers list
            div { class: "border-t border-gray-200 pt-4",
                div { class: "flex items-center justify-between mb-3",
                    p { class: "text-sm font-medium text-gray-700", "Network Status:" }
                    div { class: "flex items-center space-x-2",
                        div {
                            class: if peer_count() > 0 { "w-2 h-2 bg-green-600 rounded-full" } else { "w-2 h-2 bg-gray-300 rounded-full animate-pulse" }
                        }
                        span { class: "text-sm font-medium text-gray-900", "{peer_count()}" }
                    }
                }

                div { class: "space-y-2",
                    // This device
                    div { class: "bg-blue-50 border border-blue-200 rounded-lg p-3",
                        div { class: "flex items-center justify-between",
                            div { class: "flex items-center",
                                div { class: "mr-2",
                                    if is_coordinator {
                                        icons::Crown { class: Some("w-4 h-4 text-blue-600".to_string()) }
                                    } else {
                                        icons::Users { class: Some("w-4 h-4 text-green-600".to_string()) }
                                    }
                                }
                                span { class: "font-medium text-blue-900",
                                    "{peer_ctx.get_device_name()} (You)"
                                }
                            }
                            span { class: "text-xs text-blue-600",
                                if is_coordinator { "Coordinator" } else { "Follower" }
                            }
                        }
                        p { class: "text-xs text-blue-700 font-mono mt-1 ml-6",
                            "{room_id}"
                        }
                    }

                    // Connected peers from peer context
                    for peer in peer_ctx.get_peers() {
                        div { class: "bg-gray-50 border border-gray-200 rounded-lg p-3",
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center",
                                    icons::Smartphone { class: Some("w-4 h-4 text-gray-500 mr-2".to_string()) }
                                    span { class: "text-sm text-gray-900",
                                        "{peer.device_name}"
                                    }
                                }
                                span { class: "text-xs text-gray-500",
                                    "{peer.role}"
                                }
                            }
                            p { class: "text-xs text-gray-600 font-mono mt-1 ml-6",
                                "{peer.peer_id}"
                            }
                        }
                    }

                    // Gossipsub participants (other browsers subscribed to same topic)
                    for (idx, peer_id) in connected_peers().iter().enumerate() {
                        div { class: "bg-green-50 border border-green-200 rounded-lg p-3",
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center",
                                    div { class: "w-2 h-2 bg-green-600 rounded-full mr-2 animate-pulse" }
                                    span { class: "text-sm text-green-900 font-medium",
                                        "Browser Peer #{idx + 1}"
                                    }
                                }
                                span { class: "text-xs text-green-600 font-medium",
                                    "Active"
                                }
                            }
                            p { class: "text-xs text-green-700 font-mono mt-1 ml-4 truncate",
                                "{peer_id}"
                            }
                        }
                    }

                    // Show helpful message if no participants yet
                    if connected_peers().is_empty() && peer_count() == 0 {
                        div { class: "text-center py-4 text-gray-500 text-sm",
                            p { "No participants yet" }
                            p { class: "text-xs mt-1",
                                "Share your Peer ID to connect devices"
                            }
                        }
                    }
                }

                if peer_count() > 0 {
                    div { class: "flex items-center text-xs text-green-600 mt-3",
                        icons::CheckCircle { class: Some("w-3 h-3 mr-1".to_string()) }
                        "Ready to start DKG"
                    }
                }
            }

            // Coordinator broadcast
            if is_coordinator && peer_count() > 0 {
                div { class: "border-t border-gray-200 pt-4",
                    button {
                        class: "w-full bg-purple-600 text-white py-2 px-4 rounded-lg hover:bg-purple-700 transition text-sm flex items-center justify-center",
                        onclick: handle_broadcast_peers,
                        icons::Radio { class: Some("w-4 h-4 mr-2".to_string()) }
                        "Broadcast Peer List"
                    }
                    p { class: "text-xs text-gray-500 mt-2 text-center",
                        "Send updated peer list to all connected devices"
                    }
                }
            }

            // Debug: Broadcast test button
            div { class: "border-t border-gray-200 pt-4",
                button {
                    class: "w-full bg-orange-600 text-white py-2 px-4 rounded-lg hover:bg-orange-700 transition text-sm",
                    onclick: handle_broadcast_hello,
                    "🔊 Broadcast Hello (Debug)"
                }
                p { class: "text-xs text-gray-500 mt-2 text-center",
                    "Test gossipsub messaging"
                }
            }

            // Message display
            if !messages().is_empty() {
                div { class: "border-t border-gray-200 pt-4",
                    p { class: "text-xs font-medium text-gray-600 uppercase mb-2",
                        "Received Messages ({messages().len()})"
                    }
                    div { class: "space-y-1 max-h-32 overflow-y-auto",
                        for msg in messages() {
                            div { class: "bg-green-50 border border-green-200 rounded px-3 py-1 text-xs text-green-800 font-mono",
                                "{msg}"
                            }
                        }
                    }
                }
            }

            div { class: "flex space-x-4 pt-4",
                button {
                    class: "flex-1 bg-gray-200 text-gray-700 py-3 px-4 rounded-lg hover:bg-gray-300 transition",
                    "Back"
                }
                button {
                    class: "flex-1 bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 transition disabled:opacity-50",
                    disabled: peer_count() == 0,
                    onclick: move |_| on_next.call(()),
                    "Start DKG"
                }
            }

        }
    }
}

#[component]
fn StepDKG(is_coordinator: bool, on_next: EventHandler<()>) -> Element {
    let peer_ctx = use_context::<PeerContext>();

    rsx! {
        div { class: "space-y-6",
            h2 { class: "text-2xl font-bold text-gray-900",
                if is_coordinator { "Running DKG (Coordinator)" } else { "Running DKG (Follower)" }
            }

            p { class: "text-gray-600",
                if is_coordinator {
                    "Coordinating distributed key generation..."
                } else {
                    "Participating in DKG..."
                }
            }

            // Participants
            div { class: "bg-gray-50 border border-gray-200 rounded-lg p-4",
                p { class: "text-xs font-medium text-gray-600 uppercase mb-2",
                    "Participants ({peer_ctx.get_peer_count() + 1})"
                }
                div { class: "space-y-1",
                    div { class: "flex items-center text-sm text-gray-900",
                        if is_coordinator {
                            icons::Crown { class: Some("w-4 h-4 text-blue-600 mr-2".to_string()) }
                        } else {
                            icons::Users { class: Some("w-4 h-4 text-green-600 mr-2".to_string()) }
                        }
                        "{peer_ctx.get_device_name()} (You)"
                    }
                    for peer in peer_ctx.get_peers() {
                        div { class: "flex items-center text-sm text-gray-600",
                            icons::Smartphone { class: Some("w-4 h-4 text-gray-400 mr-2".to_string()) }
                            "{peer.device_name}"
                        }
                    }
                }
            }

            button {
                class: "w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 transition mt-6",
                onclick: move |_| on_next.call(()),
                "Complete (Test)"
            }
        }
    }
}

#[component]
fn StepComplete(on_complete: EventHandler<()>) -> Element {
    rsx! {
        div { class: "space-y-6 text-center",
            div { class: "w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mx-auto",
                svg { class: "w-8 h-8 text-green-600", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                    path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M5 13l4 4L19 7" }
                }
            }

            h2 { class: "text-2xl font-bold text-gray-900", "Wallet Created!" }

            p { class: "text-gray-600",
                "Your key share has been generated."
            }

            button {
                class: "w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 transition mt-8",
                onclick: move |_| on_complete.call(()),
                "Go to Wallet"
            }
        }
    }
}
