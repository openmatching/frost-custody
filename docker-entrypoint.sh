#!/bin/bash
# Docker entrypoint script for FROST service with SoftHSM auto-initialization
set -e

# Extract node index from config if available
NODE_INDEX=${NODE_INDEX:-0}
TOKEN_LABEL="frost-node-${NODE_INDEX}"
KEY_LABEL="frost-master-key-node${NODE_INDEX}"
PIN="${HSM_PIN:-123456}"
SO_PIN="${HSM_SO_PIN:-12345678}"

# Only initialize SoftHSM if using SoftHSM library
if [[ "$@" == *"frost-service"* ]] && [ -d "/var/lib/softhsm/tokens" ]; then
    echo "🔐 Checking SoftHSM initialization..."
    
    # Check if token already exists
    if ! softhsm2-util --show-slots 2>/dev/null | grep -q "$TOKEN_LABEL"; then
        echo "📝 Initializing SoftHSM token: $TOKEN_LABEL"
        
        # Initialize token
        softhsm2-util --init-token --slot 0 --label "$TOKEN_LABEL" \
            --so-pin "$SO_PIN" --pin "$PIN" 2>&1 | head -5
        
        # Generate EC P-256 key
        echo "🔑 Generating EC P-256 key: $KEY_LABEL"
        pkcs11-tool --module /usr/lib/softhsm/libsofthsm2.so \
            --login --pin "$PIN" \
            --keypairgen --key-type EC:prime256v1 \
            --label "$KEY_LABEL" \
            --id 01 2>&1 | grep -E "(Using slot|Key pair generated)" || true
        
        echo "✅ SoftHSM initialized: $TOKEN_LABEL / $KEY_LABEL"
    else
        echo "✅ SoftHSM token already exists: $TOKEN_LABEL"
    fi
fi

# Execute the main command
exec "$@"

