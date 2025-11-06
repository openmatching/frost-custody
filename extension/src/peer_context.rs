// Global PeerJS context using Dioxus context API
// Allows components to access P2P connection state

use std::rc::Rc;
use std::cell::RefCell;

#[derive(Clone)]
pub struct PeerContext {
    pub device_name: Rc<RefCell<String>>,
    pub peer_id: Rc<RefCell<String>>,
    pub room_id: Rc<RefCell<String>>,
    pub network: Rc<RefCell<Option<crate::p2p::NetworkManager>>>,  // Shared NetworkManager
    pub peer_object: Rc<RefCell<Option<wasm_bindgen::JsValue>>>,  // Store PeerJS object
    pub connections: Rc<RefCell<Vec<wasm_bindgen::JsValue>>>,      // Store connections
    pub connected_peers: Rc<RefCell<Vec<PeerInfo>>>,
    pub messages: Rc<RefCell<Vec<PeerMessage>>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub device_name: String,
    pub role: String,  // "coordinator" or "follower"
}

// Manual PartialEq implementation (contexts don't need real equality)
impl PartialEq for PeerContext {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.peer_id, &other.peer_id)
    }
}

#[derive(Clone, Debug)]
pub struct PeerMessage {
    pub from: String,
    pub content: String,
    pub timestamp: f64,
}

impl PeerContext {
    pub fn new() -> Self {
        Self {
            device_name: Rc::new(RefCell::new(String::new())),
            peer_id: Rc::new(RefCell::new(String::new())),
            room_id: Rc::new(RefCell::new(String::new())),
            network: Rc::new(RefCell::new(None)),
            peer_object: Rc::new(RefCell::new(None)),
            connections: Rc::new(RefCell::new(Vec::new())),
            connected_peers: Rc::new(RefCell::new(Vec::new())),
            messages: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn set_network(&self, network: crate::p2p::NetworkManager) {
        *self.network.borrow_mut() = Some(network);
    }

    pub fn get_network(&self) -> Option<crate::p2p::NetworkManager> {
        self.network.borrow().clone()
    }

    pub fn set_room_id(&self, id: String) {
        *self.room_id.borrow_mut() = id;
    }

    pub fn get_room_id(&self) -> String {
        self.room_id.borrow().clone()
    }

    pub fn set_peer_object(&self, peer: wasm_bindgen::JsValue) {
        *self.peer_object.borrow_mut() = Some(peer);
    }

    pub fn get_peer_object(&self) -> Option<wasm_bindgen::JsValue> {
        self.peer_object.borrow().clone()
    }

    pub fn add_connection(&self, conn: wasm_bindgen::JsValue) {
        self.connections.borrow_mut().push(conn);
    }

    pub fn get_connections(&self) -> Vec<wasm_bindgen::JsValue> {
        self.connections.borrow().clone()
    }

    pub fn set_device_name(&self, name: String) {
        *self.device_name.borrow_mut() = name;
    }

    pub fn get_device_name(&self) -> String {
        self.device_name.borrow().clone()
    }

    pub fn set_peer_id(&self, id: String) {
        *self.peer_id.borrow_mut() = id;
    }

    pub fn get_peer_id(&self) -> String {
        self.peer_id.borrow().clone()
    }

    pub fn add_peer(&self, peer_info: PeerInfo) {
        self.connected_peers.borrow_mut().push(peer_info);
    }

    pub fn remove_peer(&self, peer_id: &str) {
        self.connected_peers.borrow_mut().retain(|p| p.peer_id != peer_id);
    }

    pub fn get_peers(&self) -> Vec<PeerInfo> {
        self.connected_peers.borrow().clone()
    }

    pub fn get_peer_count(&self) -> usize {
        self.connected_peers.borrow().len()
    }

    pub fn clear_peers(&self) {
        self.connected_peers.borrow_mut().clear();
    }

    pub fn add_message(&self, from: String, content: String) {
        let timestamp = js_sys::Date::now();
        self.messages.borrow_mut().push(PeerMessage {
            from,
            content,
            timestamp,
        });
    }

    pub fn get_messages(&self) -> Vec<PeerMessage> {
        self.messages.borrow().clone()
    }
}

impl Default for PeerContext {
    fn default() -> Self {
        Self::new()
    }
}

