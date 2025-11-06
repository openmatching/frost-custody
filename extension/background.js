// Background service worker for FROST Wallet
// Thin JavaScript glue layer - all logic is in Rust (pkg-bg/)

console.log('🚀 Loading FROST Wallet background service worker...');

// Load Rust WASM module using importScripts (synchronous, works in Service Workers)
try {
  importScripts('pkg-bg/background.js');
  
  // Initialize WASM module
  const wasm = wasm_bindgen('pkg-bg/background_bg.wasm');
  
  // Wait for WASM to initialize
  wasm.then(() => {
    console.log('✅ Rust core loaded');
    
    // Initialize Rust background logic
    wasm_bindgen.init_background();
    
    // Chrome API glue: Icon click → Rust logic → Chrome API
    chrome.action.onClicked.addListener((tab) => {
      // Call Rust to handle business logic
      const windowId = wasm_bindgen.handle_icon_click(tab.windowId);
      
      // JavaScript glue: Call Chrome API with result from Rust
      chrome.sidePanel.open({ windowId })
        .then(() => console.log('✅ Side panel opened'))
        .catch(err => console.error('❌ Failed to open side panel:', err));
    });
    
    // Chrome API glue: Installation → Rust logic → Chrome API
    chrome.runtime.onInstalled.addListener(() => {
      // Call Rust to handle business logic
      wasm_bindgen.handle_install();
      
      // JavaScript glue: Call Chrome API
      chrome.sidePanel.setOptions({ enabled: true })
        .then(() => console.log('✅ Side panel enabled'))
        .catch(err => console.error('❌ Failed to enable side panel:', err));
    });
    
    // Message passing glue: Enable communication with main extension
    chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
      // Forward to Rust for processing
      wasm_bindgen.handle_message(message)
        .then(response => {
          sendResponse(response);
        })
        .catch(err => {
          console.error('Message handling error:', err);
          sendResponse({ error: err.toString() });
        });
      return true; // Async response
    });
    
    console.log('✅ Background worker ready (Rust core + JS glue)');
  });
  
} catch (err) {
  console.error('❌ Failed to load Rust WASM module:', err);
  console.log('⚠️ Falling back to pure JavaScript...');
  
  // Fallback: Minimal JavaScript-only implementation
  chrome.action.onClicked.addListener((tab) => {
    chrome.sidePanel.open({ windowId: tab.windowId });
  });
  
  chrome.runtime.onInstalled.addListener(() => {
    chrome.sidePanel.setOptions({ enabled: true });
  });
}

