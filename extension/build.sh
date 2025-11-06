#!/bin/bash

# Build script for FROST Wallet Chrome Extension

set -e

echo "Building FROST Wallet Extension..."

# Build main extension WASM (ES modules for browser)
echo "1. Building main extension (lib) to WASM..."
wasm-pack build --target web --out-dir pkg --release

# Build background worker binary first (to generate WASM)
echo "2. Building background worker binary..."
cargo build --target wasm32-unknown-unknown --bin background --release --no-default-features

# Package background worker with wasm-bindgen
echo "3. Packaging background worker..."
mkdir -p pkg-bg
wasm-bindgen ../target/wasm32-unknown-unknown/release/background.wasm \
  --out-dir pkg-bg \
  --target no-modules \
  --typescript

# Create dist directory
echo "4. Creating dist directory..."
mkdir -p dist
mkdir -p dist/pkg
mkdir -p dist/pkg-bg
mkdir -p dist/icons
mkdir -p dist/vendor

# Copy files
echo "5. Copying files..."
cp manifest.json dist/
cp index.html dist/
cp init.js dist/
cp background.js dist/

# Copy vendor files
echo "6. Copying vendor files..."
cp vendor/tailwind.min.css dist/vendor/
cp vendor/lucide.min.js dist/vendor/ 2>/dev/null || echo "Note: lucide.min.js not found, skipping"

# Copy WASM packages
echo "7. Copying WASM packages..."
cp -r pkg/* dist/pkg/
cp -r pkg-bg/* dist/pkg-bg/

# Create placeholder icons
echo "8. Creating placeholder icons..."
# You'll need to replace these with actual icons
touch dist/icons/icon16.png
touch dist/icons/icon48.png
touch dist/icons/icon128.png

echo "✅ Build complete!"
echo "📦 Extension is in ./dist directory"
echo ""
echo "To install in Chrome:"
echo "1. Open chrome://extensions"
echo "2. Enable 'Developer mode'"
echo "3. Click 'Load unpacked'"
echo "4. Select the ./dist directory"

