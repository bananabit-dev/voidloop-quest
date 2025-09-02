# Voidloop Quest - GitHub Copilot Instructions

Always reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.

## Project Overview

Voidloop Quest is a Rust-based multiplayer game using the Bevy game engine and Lightyear networking library. It supports both native and WebAssembly (WASM) builds and is designed to work with the Bevygap infrastructure for automatic scaling and matchmaking.

**Workspace Structure:**
- `client/` - Game client (supports both native and WASM)
- `server/` - Game server
- `shared/` - Shared game logic between client and server  
- `tools/lobby/` - CLI tool for managing Edgegap lobbies
- Root workspace manages all packages

## Working Effectively

### Quick Setup (Recommended)
Use the automated setup script that handles all dependencies:
```bash
./dev-setup.sh
```
**TIMING: 8-9 minutes. NEVER CANCEL.** Set timeout to 15+ minutes.

### Manual Setup (If automated setup fails)
Install system dependencies first:

**Ubuntu/Debian:**
```bash
sudo apt update && sudo apt install -y libudev-dev pkg-config libssl-dev build-essential curl git
```

**Fedora/CentOS/RHEL:**
```bash
sudo dnf install -y systemd-devel pkgconfig openssl-devel gcc gcc-c++ curl git
```

**macOS:**
```bash
xcode-select --install
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
brew install pkg-config openssl
```

Then install Rust toolchain:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup target add wasm32-unknown-unknown
```

### Building the Project

**Initial Build:**
```bash
cargo build
```
**TIMING: 17-18 minutes for first build. NEVER CANCEL.** Set timeout to 30+ minutes.
**SUBSEQUENT BUILDS: 30 seconds to 3 minutes for incremental changes.**

**Quick Check (for validation):**
```bash
cargo check
```
**TIMING: 30 seconds for incremental changes.**

**Release Build:**
```bash
cargo build --release
```
**TIMING: 20+ minutes. NEVER CANCEL.** Set timeout to 40+ minutes.

### Running Locally (Development Mode)

**CRITICAL:** Always use `--no-default-features` for local development to disable networking features.

**Terminal 1 - Server:**
```bash
cargo run --no-default-features -p server
```

**Terminal 2 - Client:**
```bash
cargo run --no-default-features -p client
```

The client will open a window with the game. Use A/D or Arrow keys to move, Space/W to jump.

### Building for Web (WASM)

**Install wasm-bindgen-cli (if not done by setup script):**
```bash
cargo install wasm-bindgen-cli --version 0.2.100
```

**Build WASM (using provided script):**
```bash
./build-wasm.sh
```
**TIMING: 8-9 minutes for full build, 3-4 seconds incremental. NEVER CANCEL.** Set timeout to 15+ minutes.

**Manual WASM build (if script fails):**
```bash
export RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""
cargo build --release --target wasm32-unknown-unknown --package voidloop-quest-client
wasm-bindgen --no-typescript --target web \
    --out-dir ./client/www \
    --out-name "voidloop-quest" \
    ./target/wasm32-unknown-unknown/release/voidloop-quest-client.wasm
```

**Test WASM locally:**
```bash
cd client/www && python3 -m http.server 8000
```
Then open http://localhost:8000 in your browser.

### Running Tests
```bash
cargo test
```
**TIMING: 3 seconds.** Currently no tests are defined, but the command structure is ready.

### Code Quality

**Format code:**
```bash
cargo fmt
```
**TIMING: <1 second.**

**Check formatting:**
```bash
cargo fmt --check
```
**TIMING: <1 second.**

**Linting:**
```bash
cargo clippy
```
**TIMING: 2-3 minutes. NEVER CANCEL.** Set timeout to 5+ minutes.

**Strict linting (for CI):**
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Validation Scenarios

### Complete Local Development Validation
After making changes, always validate with this complete scenario:

1. **Build and run locally:**
   ```bash
   cargo build  # TIMING: 17+ min first time, 30 sec incremental
   # Terminal 1:
   cargo run --no-default-features -p server &  # TIMING: 2-3 min compile
   # Terminal 2:
   cargo run --no-default-features -p client    # TIMING: 2-3 min compile
   ```

2. **Test basic gameplay:**
   - Client window should open showing a simple 2D platformer
   - Use A/D or Arrow keys to move the character
   - Use Space or W to jump
   - Character should interact with platforms and physics
   - Server should show connection logs in terminal
   - **CRITICAL**: Both must run without networking errors in local mode

3. **Test WASM build:**
   ```bash
   ./build-wasm.sh  # TIMING: 8+ min. NEVER CANCEL.
   cd client/www && python3 -m http.server 8000
   ```
   - Open http://localhost:8000 in browser
   - Game should load and be playable with same controls
   - WebGL2 warnings in console are normal
   - **CRITICAL**: Must verify game actually loads and runs in browser

4. **Validate code quality:**
   ```bash
   cargo fmt --check     # TIMING: <1 sec
   cargo clippy --all-targets --all-features  # TIMING: 2+ min
   ```

### **MANDATORY Pre-Commit Validation**
Always run before committing changes:
```bash
cargo fmt && cargo clippy --fix --allow-dirty && cargo build && cargo test
```

### Environment Variables

For production/network-enabled builds:
- `LIGHTYEAR_PRIVATE_KEY`: Must match matchmaker key for connect tokens
- `MATCHMAKER_URL`: URL of the matchmaker service
- `LIGHTYEAR_CERTIFICATE_DIGEST`: Only needed for WASM without bevygap

## Development Workflow Tips

- **Always use automated setup:** Run `./dev-setup.sh` first
- **Local development:** Always use `--no-default-features` flag
- **Build patience:** Initial builds take 17+ minutes - this is normal due to aggressive optimizations
- **WASM-specific:** WASM builds need specific RUSTFLAGS and wasm-bindgen-cli
- **Code style:** Run `cargo fmt` and `cargo clippy` before committing
- **Incremental builds:** After first build, subsequent builds are much faster

## Troubleshooting

### Build Errors
- **libudev error:** Install system dependencies first: `sudo apt install -y libudev-dev pkg-config`
- **wasm-bindgen not found:** Install with `cargo install wasm-bindgen-cli --version 0.2.100`
- **Long build times:** This is expected - Bevy + networking has many dependencies

### Runtime Issues
- **Connection errors when running locally:** Expected - use `--no-default-features` flag
- **Port already in use:** Kill existing processes: `pkill -f voidloop`
- **WASM not loading:** Check browser console for errors, ensure proper MIME types

## File Structure Reference

```
.
├── Cargo.toml              # Workspace configuration
├── dev-setup.sh            # Automated setup script
├── build-wasm.sh           # WASM build script
├── client/                 # Game client code
│   ├── src/
│   ├── www/               # WASM output directory
│   └── Dockerfile         # Client container
├── server/                # Game server code
│   ├── src/
│   └── Dockerfile         # Server container
├── shared/                # Shared game logic
├── tools/lobby/           # Edgegap lobby management CLI
├── DEVELOPMENT.md         # Detailed setup guide
├── WASM_BUILD.md         # WASM-specific instructions
└── README.md             # Project overview
```

## CI/CD Integration

The project has GitHub workflows for:
- **Server builds:** `.github/workflows/build-server.yaml`
- **WASM builds:** `.github/workflows/build-wasm.yaml`

Always ensure your changes pass local validation before pushing:
```bash
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings
```