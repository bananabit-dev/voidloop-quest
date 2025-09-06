# 🔧 Build Troubleshooting

This document addresses common build issues for Voidloop Quest development.

## 🚨 Runtime Issues

### 🔴 "Invalid JSON value <!DOCTYPE" when clicking "Join Room"

**Symptoms:**
- Client shows error when clicking "Join Room" button
- Browser console shows JSON parsing error with HTML content starting with `<!DOCTYPE`
- Network requests to `/hook/api/rooms` return HTML error pages instead of JSON

**Root Cause:**
The client expects REST API endpoints at `/hook/api/rooms` but no HTTP server was running to handle these requests.

**Solution:**
Ensure the `lobby-service` is running and properly configured:

1. **Local Development:**
```bash
# Start the lobby service
cargo run -p lobby-service

# Test the endpoints
curl http://localhost:3001/api/rooms
curl -X POST -H "Content-Type: application/json" \
  -d '{"host_name":"test","game_mode":"classic","max_players":4}' \
  http://localhost:3001/api/rooms
```

2. **Production Deployment:**
```bash
# Ensure lobby service is running in docker-compose
docker-compose ps
docker-compose logs lobby

# Test through proxy (after deployment)
curl https://your-domain.com/hook/api/rooms
```

**Prevention:**
- Always deploy the complete stack including the lobby service
- Verify all services are healthy before enabling client access
- Monitor lobby service logs for any startup issues

## ❌ Common Build Errors

### `libudev-sys` Build Error

**Error Message:**
```
error: failed to run custom build command for `libudev-sys v0.1.4`
...
The system library `libudev` required by crate `libudev-sys` was not found.
The file `libudev.pc` needs to be installed and the PKG_CONFIG_PATH environment variable must contain its parent directory.
```

**Solution:**
This error occurs when system development libraries are missing.

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install -y libudev-dev pkg-config
```

**Fedora/CentOS/RHEL:**
```bash
sudo dnf install -y systemd-devel pkgconfig
# or for older versions:
sudo yum install -y systemd-devel pkgconfig
```

**macOS:**
```bash
# Install Xcode command line tools if not already installed
xcode-select --install

# Install pkg-config via Homebrew
brew install pkg-config
```

### OpenSSL Build Errors

**Error Message:**
```
Could not find directory of OpenSSL installation
```

**Solution:**

**Ubuntu/Debian:**
```bash
sudo apt install -y libssl-dev
```

**Fedora/CentOS/RHEL:**
```bash
sudo dnf install -y openssl-devel
```

**macOS:**
```bash
brew install openssl
export OPENSSL_ROOT_DIR=/opt/homebrew/opt/openssl
```

### Build Tools Missing

**Error Message:**
```
linker `cc` not found
```

**Solution:**

**Ubuntu/Debian:**
```bash
sudo apt install -y build-essential
```

**Fedora/CentOS/RHEL:**
```bash
sudo dnf groupinstall -y "Development Tools"
```

**macOS:**
```bash
xcode-select --install
```

## 🚀 Quick Fix

For most build issues on Linux, run our automated setup script:

```bash
./dev-setup.sh
```

Or use the manual commands above based on your distribution.

## 📝 Reporting Issues

If you encounter build issues not covered here:

1. Check that you have the latest Rust version: `rustup update`
2. Clear your build cache: `cargo clean`
3. Try building again: `cargo build`
4. If the issue persists, please open an issue with:
   - Your operating system and version
   - Rust version (`rustc --version`)
   - Full error message
   - Output of `pkg-config --list-all | grep udev` (Linux only)

## 📖 Additional Help

- [DEVELOPMENT.md](DEVELOPMENT.md) - Complete development setup guide
- [README.md](README.md) - Quick start instructions
- [Rust Installation Guide](https://rustup.rs/) - Official Rust installation