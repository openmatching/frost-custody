#!/bin/bash
# Test script to verify all 3 nodes return the same address

set -e

echo "=== Testing Consensus Ring 3-Node Setup ==="
echo ""

# Test user ID
TEST_ID=123456

echo "Testing with user_id=$TEST_ID"
echo ""

# Check if nodes are running
echo "1. Checking if nodes are running..."
for port in 3000 3001 3002; do
    if curl -s -f http://localhost:$port/health > /dev/null 2>&1; then
        echo "✓ Node on port $port is running"
    else
        echo "✗ Node on port $port is NOT running"
        echo "  Start with: docker-compose up -d"
        exit 1
    fi
done
echo ""

# Get addresses from all nodes
echo "2. Getting multisig address from all nodes..."
ADDR0=$(curl -s http://localhost:3000/api/address?id=$TEST_ID | jq -r .address)
ADDR1=$(curl -s http://localhost:3001/api/address?id=$TEST_ID | jq -r .address)
ADDR2=$(curl -s http://localhost:3002/api/address?id=$TEST_ID | jq -r .address)

echo "Node 0 (port 3000): $ADDR0"
echo "Node 1 (port 3001): $ADDR1"
echo "Node 2 (port 3002): $ADDR2"
echo ""

# Verify all addresses match
echo "3. Verifying all nodes return identical address..."
if [ "$ADDR0" = "$ADDR1" ] && [ "$ADDR1" = "$ADDR2" ]; then
    echo "✓ SUCCESS: All nodes return the same address!"
    echo ""
    echo "Multisig address for user $TEST_ID:"
    echo "  $ADDR0"
else
    echo "✗ FAILURE: Nodes return different addresses!"
    echo "  Check that all nodes have identical xpubs in same order"
    exit 1
fi
echo ""

# Get node info
echo "4. Node information:"
for port in 3000 3001 3002; do
    INFO=$(curl -s http://localhost:$port/health)
    NODE_IDX=$(echo $INFO | jq -r .node_index)
    XPUB=$(echo $INFO | jq -r .xpub)
    echo "Node $NODE_IDX (port $port):"
    echo "  xpub: ${XPUB:0:20}..."
done
echo ""

echo "=== All tests passed! ==="
echo ""
echo "Try these commands:"
echo "  curl http://localhost:3000/docs        # Interactive API docs"
echo "  curl http://localhost:3000/api/address?id=999"

