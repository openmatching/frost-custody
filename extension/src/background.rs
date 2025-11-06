// Background service worker logic for FROST Wallet
// Minimal dependencies - no Dioxus, works with --target no-modules
// All business logic in Rust, JavaScript is just glue to Chrome APIs

use wasm_bindgen::prelude::*;
use web_sys::console;

// Dummy main for binary target
fn main() {}

/// Log helper (no wasm-logger to avoid dependencies)
fn log(msg: &str) {
    console::log_1(&JsValue::from_str(msg));
}

/// Initialize background service worker
/// Called from JavaScript glue via wasm_bindgen
#[wasm_bindgen]
pub fn init_background() {
    log("🚀 FROST Wallet background service initialized (Rust core)");
}

/// Handle extension icon click
/// Returns the window ID that should open the side panel
#[wasm_bindgen]
pub fn handle_icon_click(window_id: u32) -> u32 {
    log(&format!("Extension icon clicked (window: {})", window_id));
    
    // Business logic here (for now, just pass through)
    // Future: Could check if wallet is locked, show different panels, etc.
    
    window_id
}

/// Handle extension installation
#[wasm_bindgen]
pub fn handle_install() {
    log("Extension installed or updated");
    
    // Business logic here
    // Future: Initialize storage, set default settings, etc.
}

/// Handle message from main extension
/// This enables communication between UI and background
#[wasm_bindgen]
pub fn handle_message(message: JsValue) -> JsValue {
    log(&format!("Received message from extension: {:?}", message));
    
    // Parse message type and route to appropriate handler
    // Future: Handle signing requests, P2P messages, etc.
    
    JsValue::from_str("acknowledged")
}

// Future business logic functions to add:
// - handle_signing_request(tx_data: Vec<u8>) -> Result<Signature, Error>
// - maintain_p2p_connections() -> Result<(), Error>
// - process_offline_messages() -> Result<(), Error>
// - auto_lock_wallet(timeout_ms: u32) -> Result<(), Error>

