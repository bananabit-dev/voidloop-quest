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

## Manual TLS Configuration for Rust Developers

If you need to manually configure TLS connections to NATS in your Rust application (outside of the automatic bevygap handling), you can load the root CA certificate directly in your code.

### Certificate Placement

The root CA certificate (used to sign the NATS server certificate) should be placed on the client filesystem, for example:
- `/etc/ssl/certs/nats_ca.crt`
- `/tmp/nats_ca.pem`
- Any accessible path in your container/environment

### Rust Code Example

Here's how to load the root CA certificate and configure TLS for the async_nats client:

```rust
use async_nats::{Client, ConnectOptions};
use rustls_pki_types::CertificateDer;
use std::fs::File;
use std::io::BufReader;

async fn connect_to_nats_with_tls() -> Result<Client, Box<dyn std::error::Error>> {
    // Load the root CA certificate from filesystem
    let cert_file = File::open("/etc/ssl/certs/nats_ca.crt")?;
    let mut reader = BufReader::new(cert_file);
    let certs = rustls_pemfile::certs(&mut reader)?;
    
    // Create a root certificate store and add our CA
    let mut root_store = rustls::RootCertStore::empty();
    for cert in certs {
        root_store.add(cert)?;
    }
    
    // Configure TLS client
    let tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    
    // Connect to NATS with TLS configuration
    let client = async_nats::connect_with_options(
        "tls://nats.example.com:4222",
        ConnectOptions::new()
            .tls_config(tls_config)
            .user_and_password("gameserver", "your_password")
    ).await?;
    
    Ok(client)
}
```

### Alternative: Using native-tls (simpler API, as requested in issue)

For simpler TLS configuration, you can use the native-tls approach which matches the pattern suggested in the issue:

```rust
use async_nats::{Client, ConnectOptions};
use native_tls::{Certificate, TlsConnector};
use std::fs;

async fn connect_to_nats_with_native_tls() -> Result<Client, Box<dyn std::error::Error>> {
    // Load the root CA certificate
    let cert = fs::read("/etc/ssl/certs/nats_ca.crt")?;
    let tls_connector = TlsConnector::builder()
        .add_root_certificate(Certificate::from_pem(&cert)?)
        .build()?;
    
    // Connect to NATS with TLS
    let client = async_nats::connect_with_options(
        "tls://nats.example.com:4222",
        ConnectOptions::new()
            .tls_connector(tls_connector)
            .user_and_password("gameserver", "your_password")
    ).await?;
    
    Ok(client)
}
```

### Startup Configuration

**Important:** This TLS configuration should be done at startup, before making any NATS connections. Here's an example of integrating it into a Bevy application:

```rust
use bevy::prelude::*;

#[derive(Resource)]
struct NatsClient(async_nats::Client);

fn setup_nats_connection(mut commands: Commands) {
    // This should run in an async context or use a runtime
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client = rt.block_on(connect_to_nats_with_tls()).unwrap();
    commands.insert_resource(NatsClient(client));
}

fn main() {
    App::new()
        .add_systems(Startup, setup_nats_connection)
        .run();
}
```

### Key Points

- The root CA certificate is needed on **any client** connecting to NATS with TLS, not just the NATS server itself
- Certificate loading must happen before establishing NATS connections
- The CA certificate file path must be accessible to your application
- For containerized deployments, ensure the certificate is properly mounted or copied into the container

## For Game Developers

When deploying to Edgegap:

1. Build and push your server image
2. Create an Edgegap application
3. Use the `set-caroot-argument.sh` script to configure certificate passing
4. Deploy your application - the certificates will be automatically configured

### Automatic Handling in bevygap

The bevygap server plugin automatically handles NATS TLS configuration when the `NATS_CA` environment variable is set (which happens automatically when using the `--ca_contents` command line argument described above). Manual TLS configuration is only needed if you're implementing custom NATS client logic outside of bevygap.