# WASM Build Instructions

This document describes how to build and run the WASM version of voidloop-quest-client.

## Quick Start

Use the provided build script:

```bash
./build-wasm.sh
```

This will:
1. Build the WASM binary with proper RUSTFLAGS
2. Generate wasm-bindgen files
3. Output everything to `client/www/`

## Manual Build

If you prefer to build manually:

```bash
# Set proper RUSTFLAGS for WASM compilation
export RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""

# Build the WASM binary (note: --no-default-features is required for WASM)
cargo build --release --target wasm32-unknown-unknown --package voidloop-quest-client --no-default-features

# Generate WASM bindings
wasm-bindgen --no-typescript --target web \
    --out-dir ./client/www \
    --out-name "voidloop-quest" \
    ./target/wasm32-unknown-unknown/release/voidloop-quest-client.wasm
```

## Local Testing

To test the WASM build locally:

```bash
cd client/www
python3 -m http.server 8000
```

Then open http://localhost:8000 in your browser.

## Prerequisites

- Rust with `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- wasm-bindgen-cli version 0.2.100: `cargo install wasm-bindgen-cli --version 0.2.100`

## Notes

- **Important**: WASM builds must use `--no-default-features` to exclude networking features (bevygap) that require tokio with unsupported features for WASM
- The client expects a canvas element with ID `#game`
- The application will attempt to connect to a matchmaker WebSocket (expected to fail in local development)
- WebGL2 warnings in the console are normal and don't indicate errors
- The application uses proper CORS headers and MIME types for production deployment