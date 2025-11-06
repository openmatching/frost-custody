// Extension initialization script
// Loads the WASM module and starts the Dioxus app

import init from './pkg/frost_wallet_extension.js';

init().then(() => {
    console.log('FROST Wallet initialized');
}).catch(err => {
    console.error('Failed to initialize FROST Wallet:', err);
});

