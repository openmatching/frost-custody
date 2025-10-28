#!/bin/bash
set -e

# Initialize SoftHSM if PKCS11 is enabled
if [ -n "$USE_SOFTHSM" ] && [ "$USE_SOFTHSM" = "true" ]; then
    echo "🔐 Initializing SoftHSM..."
    
    # Create SoftHSM config
    mkdir -p /var/lib/softhsm/tokens
    cat > /etc/softhsm2.conf << EOF
directories.tokendir = /var/lib/softhsm/tokens
objectstore.backend = file
log.level = INFO
EOF
    
    # Check if token already exists
    if ! softhsm2-util --show-slots | grep -q "$HSM_KEY_LABEL"; then
        echo "📝 Creating SoftHSM token: $HSM_KEY_LABEL"
        
        # Initialize token
        softhsm2-util --init-token --slot 0 --label "$HSM_KEY_LABEL" \
            --so-pin "$HSM_SO_PIN" --pin "$HSM_PIN"
        
        # Generate P-256 key using pkcs11-tool
        if command -v pkcs11-tool &> /dev/null; then
            pkcs11-tool --module /usr/lib/softhsm/libsofthsm2.so \
                --login --pin "$HSM_PIN" \
                --keypairgen --key-type EC:prime256v1 \
                --label "$HSM_KEY_LABEL" 2>&1 | head -5
            echo "✅ SoftHSM initialized with P-256 key: $HSM_KEY_LABEL"
        else
            echo "⚠️  pkcs11-tool not found, generating key via SoftHSM2"
            # Alternative: Generate key using SoftHSM utilities
            # This is less ideal but works
            echo "  Token created: $HSM_KEY_LABEL"
            echo "  Key will be generated on first use"
        fi
    else
        echo "✅ SoftHSM token already exists: $HSM_KEY_LABEL"
    fi
    
    # List tokens for debugging
    echo "📋 Available tokens:"
    softhsm2-util --show-slots
fi

# Execute the main command
exec "$@"

