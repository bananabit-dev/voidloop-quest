#!/bin/bash

# Build script for WASM version of voidloop-quest-client
# Based on the Dockerfile configuration

set -e

echo "🔧 Building WASM for voidloop-quest-client..."

# Set proper RUSTFLAGS for WASM compilation
export RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""

# Build the WASM binary
echo "📦 Building WASM binary..."
cargo build --release --target wasm32-unknown-unknown --package voidloop-quest-client

# Generate WASM bindings
echo "🔗 Generating WASM bindings..."
wasm-bindgen --no-typescript --target web \
    --out-dir ./client/www \
    --out-name "voidloop-quest" \
    ./target/wasm32-unknown-unknown/release/voidloop-quest-client.wasm

echo "✅ WASM build complete!"
echo "📁 Output files:"
echo "   - client/www/index.html"
echo "   - client/www/voidloop-quest.js"
echo "   - client/www/voidloop-quest_bg.wasm"
echo ""
echo "🌐 To test locally:"
echo "   cd client/www && python3 -m http.server 8000"
echo "   Then open http://localhost:8000"