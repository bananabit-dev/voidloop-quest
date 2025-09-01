#!/usr/bin/env bash
# Development Environment Setup Script for Voidloop Quest
# Installs required system dependencies for building the project on Linux/macOS

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
info() { echo -e "${BLUE}[INFO]${NC} $*"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
warning() { echo -e "${YELLOW}[WARNING]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; }

command_exists() { command -v "$1" >/dev/null 2>&1; }

# Detect OS
detect_os() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if command_exists apt-get; then
            echo "ubuntu"
        elif command_exists dnf; then
            echo "fedora"
        elif command_exists yum; then
            echo "centos"
        else
            echo "linux-unknown"
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        echo "macos"
    elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
        echo "windows"
    else
        echo "unknown"
    fi
}

# Install system dependencies
install_dependencies() {
    local os="$1"
    
    info "Installing system dependencies for $os..."
    
    case "$os" in
        ubuntu)
            info "Updating package list..."
            sudo apt-get update -qq
            
            info "Installing build dependencies..."
            sudo apt-get install -y \
                libudev-dev \
                pkg-config \
                libssl-dev \
                build-essential \
                curl \
                git
            ;;
        fedora)
            info "Installing build dependencies..."
            sudo dnf install -y \
                systemd-devel \
                pkgconfig \
                openssl-devel \
                gcc \
                gcc-c++ \
                curl \
                git
            ;;
        centos)
            info "Installing build dependencies..."
            sudo yum install -y \
                systemd-devel \
                pkgconfig \
                openssl-devel \
                gcc \
                gcc-c++ \
                curl \
                git
            ;;
        macos)
            if ! command_exists brew; then
                warning "Homebrew not found. Installing Homebrew..."
                /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
            fi
            
            info "Installing build dependencies..."
            brew install pkg-config openssl
            ;;
        windows)
            warning "Windows detected. Please use WSL2 with Ubuntu and run this script inside WSL."
            warning "Alternatively, see DEVELOPMENT.md for manual setup instructions."
            exit 1
            ;;
        *)
            error "Unsupported operating system: $os"
            error "Please see DEVELOPMENT.md for manual setup instructions."
            exit 1
            ;;
    esac
}

# Install Rust if not present
install_rust() {
    if command_exists rustc; then
        success "Rust is already installed ($(rustc --version))"
        return
    fi
    
    info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    success "Rust installed successfully"
}

# Install WASM target
install_wasm_target() {
    info "Installing WASM target..."
    rustup target add wasm32-unknown-unknown
    success "WASM target installed"
}

# Install trunk for WASM builds
install_trunk() {
    if command_exists trunk; then
        success "Trunk is already installed"
        return
    fi
    
    info "Installing trunk for WASM builds (this may take a few minutes)..."
    info "Note: You can skip this and install trunk later with 'cargo install trunk'"
    
    # Give user option to skip trunk installation
    if [[ "${SKIP_TRUNK:-}" == "1" ]]; then
        warning "Skipping trunk installation (SKIP_TRUNK=1)"
        return
    fi
    
    cargo install trunk
    success "Trunk installed successfully"
}

# Test build
test_build() {
    info "Testing project build..."
    if cargo check --quiet; then
        success "Project builds successfully!"
    else
        error "Build failed. Please check the output above for errors."
        exit 1
    fi
}

# Main setup function
main() {
    echo -e "${BLUE}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                                                              ║"
    echo "║     🛠️  Voidloop Quest Development Environment Setup         ║"
    echo "║                                                              ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    
    # Detect OS
    local os
    os=$(detect_os)
    info "Detected OS: $os"
    
    # Install dependencies
    install_dependencies "$os"
    
    # Install Rust toolchain
    install_rust
    
    # Make sure we have cargo in PATH
    if ! command_exists cargo; then
        export PATH="$HOME/.cargo/bin:$PATH"
        if ! command_exists cargo; then
            error "Cargo not found in PATH. Please restart your terminal and try again."
            exit 1
        fi
    fi
    
    # Install WASM target and trunk
    install_wasm_target
    install_trunk
    
    # Test the build
    test_build
    
    echo -e "${GREEN}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                                                              ║"
    echo "║     ✅ Development environment setup complete!               ║"
    echo "║                                                              ║"
    echo "║     Next steps:                                              ║"
    echo "║     • cargo run --no-default-features -p server             ║"
    echo "║     • cargo run --no-default-features -p client             ║"
    echo "║                                                              ║"
    echo "║     💡 Tips:                                                 ║"
    echo "║     • Run with SKIP_TRUNK=1 to skip trunk installation      ║"
    echo "║     • Install trunk later: cargo install trunk              ║"
    echo "║                                                              ║"
    echo "║     📖 See DEVELOPMENT.md for detailed instructions          ║"
    echo "║                                                              ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Check if running from the correct directory
if [[ ! -f "Cargo.toml" ]] || [[ ! -d "client" ]] || [[ ! -d "server" ]]; then
    error "This script must be run from the voidloop-quest project root directory"
    error "Current directory: $(pwd)"
    exit 1
fi

# Run main setup
main "$@"