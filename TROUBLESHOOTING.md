# 🔧 Build Troubleshooting

This document addresses common build issues for Voidloop Quest development.

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