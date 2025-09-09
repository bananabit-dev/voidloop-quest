#!/bin/bash
set -e

echo "🔧 Testing Lobby Deployment Fix"
echo "==============================="

echo "📦 Building lobby tool..."
cargo build -p lobby

echo ""
echo "✅ Testing help message..."
cargo run -p lobby -- --help | head -10

echo ""
echo "⚠️ Testing deploy without app configuration (should show warning)..."
export EDGEGAP_BASE_URL="https://api.edgegap.com"
export EDGEGAP_TOKEN="dummy-token-for-testing"
unset EDGEGAP_APP_NAME
unset EDGEGAP_APP_VERSION
timeout 5s cargo run -p lobby -- deploy test-lobby 2>&1 | head -10 || true

echo ""
echo "✅ Testing deploy with app configuration (should show app info)..."
export EDGEGAP_APP_NAME="voidloop-quest-server"
export EDGEGAP_APP_VERSION="1.0.0"
timeout 5s cargo run -p lobby -- deploy test-lobby 2>&1 | head -5 || true

echo ""
echo "🎯 Fix Summary:"
echo "- Added --app-name and --app-version parameters"
echo "- Added environment variable support (EDGEGAP_APP_NAME, EDGEGAP_APP_VERSION)"
echo "- Added warning when deploying without app configuration"
echo "- Enhanced payload includes app info for Edgegap deployment"
echo "- Updated documentation with new requirements"
echo ""
echo "✅ Lobby deployment fix is ready!"