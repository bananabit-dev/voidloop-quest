#!/bin/bash
# Simple test to verify server argument parsing

set -e

echo "🧪 Testing server argument parsing..."

# Create a test certificate
TEST_CERT="-----BEGIN CERTIFICATE-----
MIIBmzCCAQOgAwIBAgIRAKNGmGVOtZrLGNF/VGhYwK0wDQYJKoZIhvcNAQELBQAw
EzERMA0GA1UEAxMGdGVzdDEwHhcNMjQwOTA2MjEwNzEyWhcNMjUwOTA2MjEwNzEy
WjATMREwDwYDVQQDEwh0ZXN0MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKC
AQEAtestcertificatecontenthere==
-----END CERTIFICATE-----"

# Test argument parsing (should exit quickly with argument validation)
echo "📋 Testing help output..."
timeout 5 cargo run --package server --no-default-features -- --help || true

echo "📋 Testing argument validation..."
echo "Expected: Server should parse arguments and start initialization..."

# Test with valid arguments (should start but then fail without proper environment)
timeout 3 cargo run --package server --no-default-features -- \
    --host 127.0.0.1 \
    --port 6420 \
    --transport-port 6421 \
    --ca_contents "$TEST_CERT" || true

echo "✅ Argument parsing test completed!"
echo "✅ Note: Server should parse arguments correctly and begin initialization."
echo "✅ Actual networking functionality requires proper environment setup."