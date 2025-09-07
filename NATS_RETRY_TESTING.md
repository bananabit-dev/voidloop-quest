# NATS Retry Logic - Testing Guide

This document describes the NATS retry logic implementation and how to test it.

## New Features

### Command Line Argument
- `--nats-retry-count <count>`: Number of NATS connection retry attempts (default: 5)

### Environment Variable
- `NATS_RETRY_COUNT`: Override the retry count from environment (takes precedence over command line)

## How It Works

1. **Detection**: The server checks for NATS environment variables (`NATS_HOST` or `NATS_USER`)
2. **Connection Test**: If NATS environment is detected, it attempts to connect before starting the main server
3. **Retry Logic**: On failure, it retries up to the specified count with incremental backoff (1s, 2s, 3s, 4s, 5s)
4. **Failure Handling**: After all retries fail, logs "Failed to connect to NATS" and exits with error code 1

## Testing

### Test 1: Without NATS Environment (Should Skip)
```bash
cargo run --no-default-features -p server -- --help
# Should show the new --nats-retry-count argument
```

### Test 2: With Invalid NATS Host (Should Retry and Fail)
```bash
NATS_HOST=nonexistent.host NATS_USER=test cargo run --features bevygap -p server -- --nats-retry-count 2
# Should show:
# 🔌 NATS environment variables detected, testing connection...
# 🔌 Testing NATS connection to: nats://nonexistent.host:4222
# 🔄 NATS retry attempts configured: 2
# 🔄 NATS connection attempt 1/2
# ❌ NATS connection attempt 1 failed: [error]
# ⏱️  Waiting 1s before next attempt...
# 🔄 NATS connection attempt 2/2  
# ❌ NATS connection attempt 2 failed: [error]
# ❌ Failed to connect to NATS: Failed to connect to NATS after 2 attempts: [error]
# ❌ Server startup aborted due to NATS connection failure
# [exits with code 1]
```

### Test 3: With Environment Override
```bash
NATS_HOST=badhost NATS_USER=test NATS_RETRY_COUNT=3 cargo run --features bevygap -p server -- --nats-retry-count 5
# Should use 3 retries (from environment) instead of 5 (from command line)
```

## Integration

The retry logic only activates when:
1. The `bevygap` feature is enabled
2. At least one NATS environment variable is present (`NATS_HOST` or `NATS_USER`)

This ensures backward compatibility and doesn't affect local development setups without NATS.