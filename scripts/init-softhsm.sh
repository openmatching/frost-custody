#!/bin/bash
# Initialize SoftHSM token and generate master key for FROST node
#
# Usage:
#   ./scripts/init-softhsm.sh node0          # Create token "frost-node-0"
#   ./scripts/init-softhsm.sh node1 987654   # With custom PIN
#
# This script:
#   1. Initializes a SoftHSM token
#   2. Generates an EC P-256 key pair inside the HSM
#   3. The key never leaves the HSM (even in SoftHSM)
#   4. Same key used for dev → prod migration

set -e

# Configuration
NODE_NAME="${1:-node0}"
PIN="${2:-123456}"
SO_PIN="${3:-12345678}"
TOKEN_LABEL="frost-${NODE_NAME}"
KEY_LABEL="frost-master-key-${NODE_NAME}"

# Detect OS and set library path
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    PKCS11_LIB="/usr/lib/softhsm/libsofthsm2.so"
    if [ ! -f "$PKCS11_LIB" ]; then
        PKCS11_LIB="/usr/lib/x86_64-linux-gnu/softhsm/libsofthsm2.so"
    fi
elif [[ "$OSTYPE" == "darwin"* ]]; then
    PKCS11_LIB="/opt/homebrew/lib/softhsm/libsofthsm2.so"
    if [ ! -f "$PKCS11_LIB" ]; then
        PKCS11_LIB="/usr/local/lib/softhsm/libsofthsm2.so"
    fi
else
    echo "❌ Unsupported OS: $OSTYPE"
    exit 1
fi

if [ ! -f "$PKCS11_LIB" ]; then
    echo "❌ SoftHSM library not found at $PKCS11_LIB"
    echo ""
    echo "Install SoftHSM:"
    echo "  Ubuntu/Debian: sudo apt-get install softhsm2"
    echo "  macOS:         brew install softhsm"
    exit 1
fi

echo "═══════════════════════════════════════════════════════════"
echo "  FROST MPC - SoftHSM Initialization"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Node:        $NODE_NAME"
echo "Token label: $TOKEN_LABEL"
echo "Key label:   $KEY_LABEL"
echo "PIN:         $PIN (change in production!)"
echo "SO PIN:      $SO_PIN (Security Officer PIN)"
echo "Library:     $PKCS11_LIB"
echo ""

# Step 1: Check if token already exists
echo "📋 Step 1: Checking for existing token..."
if softhsm2-util --show-slots | grep -q "$TOKEN_LABEL"; then
    echo "⚠️  Token '$TOKEN_LABEL' already exists!"
    echo ""
    read -p "Delete and recreate? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "🗑️  Deleting existing token..."
        TOKEN_SERIAL=$(softhsm2-util --show-slots | grep -A 5 "$TOKEN_LABEL" | grep "Serial number" | awk '{print $NF}')
        softhsm2-util --delete-token --serial "$TOKEN_SERIAL"
    else
        echo "Aborted."
        exit 0
    fi
fi

# Step 2: Initialize token
echo ""
echo "🔐 Step 2: Initializing SoftHSM token..."
softhsm2-util --init-token --slot 0 \
    --label "$TOKEN_LABEL" \
    --so-pin "$SO_PIN" \
    --pin "$PIN"

# Step 3: Find the slot number
echo ""
echo "🔍 Step 3: Finding token slot..."
SLOT=$(softhsm2-util --show-slots | grep -B 1 "$TOKEN_LABEL" | grep "Slot" | awk '{print $2}')
echo "   Token is in slot: $SLOT"

# Step 4: Generate EC P-256 key pair
echo ""
echo "🔑 Step 4: Generating EC P-256 master key..."
pkcs11-tool --module "$PKCS11_LIB" \
    --slot "$SLOT" \
    --login --pin "$PIN" \
    --keypairgen \
    --key-type EC:prime256v1 \
    --label "$KEY_LABEL" \
    --id 01

# Step 5: Verify key generation
echo ""
echo "✅ Step 5: Verifying key..."
echo ""
echo "Public keys in token:"
pkcs11-tool --module "$PKCS11_LIB" \
    --slot "$SLOT" \
    --list-objects --type pubkey

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  ✅ Success! SoftHSM initialized for $NODE_NAME"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Configuration for config.toml:"
echo "---"
echo "[node.key_provider]"
echo "pkcs11_library = \"$PKCS11_LIB\""
echo "slot = $SLOT"
echo "pin = \"$PIN\"  # Use environment variable in production!"
echo "key_label = \"$KEY_LABEL\""
echo "---"
echo ""
echo "📝 Next steps:"
echo "   1. Copy above config to your node's config.toml"
echo "   2. Start node: cargo run --release --bin frost-service"
echo "   3. Unlock HSM: curl -X POST http://localhost:4000/api/hsm/unlock \\"
echo "                       -H 'Content-Type: application/json' \\"
echo "                       -d '{\"pin\":\"$PIN\"}'"
echo ""
echo "💡 For production:"
echo "   - Use strong PIN (not 123456!)"
echo "   - Store PIN in environment variable"
echo "   - Backup token files from ~/.softhsm2/"
echo "   - Or migrate to YubiKey/real HSM using same key label"
echo ""

