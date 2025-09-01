# 🛠️ Development Setup Guide

This guide helps you set up a local development environment for Voidloop Quest.

## Prerequisites

### System Requirements
- **OS**: Linux (Ubuntu/Debian), macOS, or Windows with WSL2
- **Rust**: Latest stable version (install via [rustup](https://rustup.rs/))
- **Node.js**: v16+ (for web client development)
- **Git**: For version control

### Required System Dependencies

#### Ubuntu/Debian
```bash
sudo apt update
sudo apt install -y \
    libudev-dev \
    pkg-config \
    libssl-dev \
    build-essential \
    curl \
    git
```

#### Fedora/CentOS/RHEL
```bash
sudo dnf install -y \
    systemd-devel \
    pkgconfig \
    openssl-devel \
    gcc \
    gcc-c++ \
    curl \
    git
```

#### macOS
```bash
# Install Xcode command line tools
xcode-select --install

# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install required packages
brew install pkg-config openssl
```

#### Windows (WSL2)
Use the Ubuntu/Debian instructions within your WSL2 environment.

## Installation Steps

### 1. Clone the Repository
```bash
git clone https://github.com/bananabit-dev/voidloop-quest.git
cd voidloop-quest
```

### 2. Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 3. Install WASM Target (for web client)
```bash
rustup target add wasm32-unknown-unknown
```

### 4. Install trunk (for WASM builds)
```bash
cargo install trunk
```

### 5. Verify Installation
```bash
# Test that the project compiles
cargo check

# Build all components
cargo build
```

## Development Workflow

### Running the Game Locally (No Networking)

For local development without networking features:

```bash
# Terminal 1: Run the server
cargo run --no-default-features -p server

# Terminal 2: Run the client
cargo run --no-default-features -p client
```

### Building for Web (WASM)
```bash
cd client
trunk build --release
```

The web build will be available in `client/dist/`.

### Running with Docker (Local Development)
```bash
# Build and run with Docker Compose
docker-compose -f docker-compose.localdev.yml up --build
```

This will start:
- Client on http://localhost:8080
- Server on UDP port 6420, TCP port 6421

### Running Tests
```bash
# Run all tests
cargo test

# Run tests for a specific package
cargo test -p server
cargo test -p client
cargo test -p shared
```

### Code Formatting and Linting
```bash
# Format code
cargo fmt

# Check for common issues
cargo clippy

# Check without building
cargo check
```

## Troubleshooting

### Build Errors

#### `libudev-sys` Error
```
error: failed to run custom build command for `libudev-sys`
The system library `libudev` required by crate `libudev-sys` was not found.
```

**Solution**: Install the development dependencies listed above, particularly `libudev-dev` and `pkg-config`.

#### OpenSSL Errors
If you encounter OpenSSL-related build errors:

**Ubuntu/Debian**:
```bash
sudo apt install libssl-dev
```

**macOS**:
```bash
brew install openssl
export OPENSSL_ROOT_DIR=/opt/homebrew/opt/openssl
```

#### WASM Build Issues
If `trunk build` fails:
1. Ensure you have the WASM target installed: `rustup target add wasm32-unknown-unknown`
2. Check that trunk is installed: `cargo install trunk`
3. Clear cache: `trunk clean` then `trunk build`

### Runtime Issues

#### Connection Failed Errors
When running locally without the full Bevygap infrastructure, connection errors are expected. Use the `--no-default-features` flag to disable networking features for local development.

#### Port Already in Use
If you get "port already in use" errors:
- Check if another instance is running: `ps aux | grep voidloop`
- Kill existing processes: `pkill -f voidloop`
- Use different ports by setting environment variables

## Environment Variables

For local development, you can create a `.env` file:

```bash
# Optional: Set log level
RUST_LOG=debug

# Optional: Custom server settings
SERVER_HOST=127.0.0.1
SERVER_PORT=6420
TRANSPORT_PORT=6421
MAX_PLAYERS=16
```

## Next Steps

- Read the [deployment guide](docs/deployment.md) for production setup
- Check out the [troubleshooting guide](setup.sh) for common production issues
- Join our community for support and contributions

## Contributing

Before contributing:
1. Ensure all tests pass: `cargo test`
2. Format your code: `cargo fmt`
3. Check for issues: `cargo clippy`
4. Update documentation if needed