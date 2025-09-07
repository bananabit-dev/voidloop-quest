# NATS Certificate Configuration for Edgegap

This document describes how to configure NATS certificates for Edgegap deployments, working around the 255-byte environment variable limitation.

## Problem

Edgegap has a 255-byte limit on environment variables, but NATS certificate contents are typically much larger. This prevents setting `NATS_CA_CONTENTS` directly as an environment variable.

## Solution

The server now supports a `--ca_contents` command line argument that accepts the full certificate contents and writes them to a temporary file, then sets the `NATS_CA` environment variable to point to that file.

## Usage

### Using the Utility Script

The `utils/set-caroot-argument.sh` script configures an Edgegap application to pass certificate contents as command line arguments:

```bash
export EDGEGAP_API_KEY="token-xxxxx-xxxxx-xxxxx"
./utils/set-caroot-argument.sh "your-app-name" "1" "/path/to/rootCA.pem"
```

This will update your Edgegap application configuration to pass `--ca_contents '<certificate-contents>'` to the server on startup.

### Server Arguments

The server now supports these command line arguments:

- `--host <address>` - Host address to bind to (default: 0.0.0.0)
- `--port <port>` - Port to listen on (default: 6420)
- `--transport-port <port>` - Transport port for WebTransport (default: 6421)
- `--transport <type>` - Transport type: websocket or webtransport (default: websocket)
- `--ca_contents <certificate>` - NATS certificate contents (for Edgegap workaround)

### Example

```bash
./server --host 0.0.0.0 --port 6420 --transport-port 6421 --ca_contents "-----BEGIN CERTIFICATE-----
MIIBmzCCAQOgAwIBAgIRAKNGmGVOtZrLGNF...
-----END CERTIFICATE-----"
```

The server will:
1. Write the certificate contents to `/tmp/nats_ca.pem`
2. Set `NATS_CA=/tmp/nats_ca.pem` environment variable
3. Use this for NATS connections
4. Test NATS connection with retry logic before starting

## NATS Connection Retry

The server includes automatic retry logic for NATS connections:

- **Default retries**: 5 attempts with exponential backoff (1s, 2s, 3s, 4s, 5s)
- **Command line**: `--nats-retry-count <count>` to override default
- **Environment variable**: `NATS_RETRY_COUNT=<count>` for runtime configuration
- **Behavior**: If all retries fail, the server logs "Failed to connect to NATS" and exits

The retry logic only activates when NATS environment variables are detected (`NATS_HOST` or `NATS_USER`).

## For Game Developers

When deploying to Edgegap:

1. Build and push your server image
2. Create an Edgegap application
3. Use the `set-caroot-argument.sh` script to configure certificate passing
4. Deploy your application - the certificates will be automatically configured