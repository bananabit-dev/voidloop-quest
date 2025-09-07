#!/bin/bash

echo "=== Testing NATS retry logic ==="

echo "1. Testing server startup without NATS environment variables (should skip NATS test):"
echo "Command: cargo run --no-default-features -p server --help"
cargo run --no-default-features -p server -- --help 2>/dev/null | grep -A2 -B2 nats-retry-count

echo ""
echo "2. Showing new argument in help:"
cargo run --no-default-features -p server -- --help 2>/dev/null | grep -A1 "nats-retry-count"

echo ""
echo "3. Testing with invalid NATS host (should trigger retry logic and fail after 2 attempts):"
echo "Setting NATS_HOST=nonexistent.host NATS_USER=test and nats-retry-count=2"
echo "This should show retry attempts and then exit with error."

export NATS_HOST=nonexistent.host
export NATS_USER=test
timeout 30 cargo run --features bevygap -p server -- --nats-retry-count 2 2>&1 | grep -E "(🔌|🔄|❌|✅)" | head -10

echo ""
echo "=== Test complete ==="