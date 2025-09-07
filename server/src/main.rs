use bevy::prelude::*;
use clap::Parser;
use server_plugin::ServerPlugin;
use std::env;
#[cfg(feature = "bevygap")]
use std::time::Duration;

mod build_info;
mod server_plugin;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Host address to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 6420)]
    port: u16,

    /// Transport port for WebTransport
    #[arg(long, default_value_t = 6421)]
    transport_port: u16,

    /// Transport type (websocket or webtransport)
    #[arg(short, long, default_value = "websocket")]
    transport: String,

    /// NATS certificate contents (for Edgegap deployment workaround)
    #[arg(long)]
    ca_contents: Option<String>,

    /// Number of NATS connection retry attempts (default: 5)
    #[arg(long, default_value_t = 5)]
    nats_retry_count: u32,
}

/// Test NATS connection with retry logic
#[cfg(feature = "bevygap")]
async fn test_nats_connection_with_retry(retry_count: u32) -> Result<(), String> {
    use async_nats::ConnectOptions;
    
    // Get NATS connection parameters from environment
    let nats_host = env::var("NATS_HOST").unwrap_or_else(|_| "localhost".to_string());
    let nats_port = env::var("NATS_PORT").unwrap_or_else(|_| "4222".to_string());
    let nats_user = env::var("NATS_USER").unwrap_or_else(|_| "gameserver".to_string());
    let nats_password = env::var("NATS_PASSWORD").unwrap_or_default();
    let nats_ca = env::var("NATS_CA").ok();
    
    // Construct NATS URL
    let scheme = if nats_ca.is_some() { "tls" } else { "nats" };
    let nats_url = format!("{}://{}:{}", scheme, nats_host, nats_port);
    
    info!("🔌 Testing NATS connection to: {}", nats_url);
    info!("🔌 NATS User: {}", nats_user);
    if nats_ca.is_some() {
        info!("🔌 NATS TLS enabled with CA certificate");
    }
    
    // Override retry count from environment if set
    let retry_count = env::var("NATS_RETRY_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(retry_count);
    
    info!("🔄 NATS retry attempts configured: {}", retry_count);

    for attempt in 1..=retry_count {
        info!("🔄 NATS connection attempt {}/{}", attempt, retry_count);
        
        let options = ConnectOptions::new()
            .user_and_password(nats_user.clone(), nats_password.clone())
            .connection_timeout(Duration::from_secs(5))
            .request_timeout(Some(Duration::from_secs(3)));
        
        // For TLS connections, we'll rely on the system's CA store or NATS_CA env var
        // The actual TLS configuration is typically handled by the NATS client itself
        
        // Attempt to connect
        match async_nats::connect_with_options(&nats_url, options).await {
            Ok(client) => {
                info!("✅ NATS connection successful on attempt {}", attempt);
                
                // Test basic functionality with a flush
                match client.flush().await {
                    Ok(_) => {
                        info!("✅ NATS ping successful");
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("❌ NATS ping failed: {}", e);
                        if attempt == retry_count {
                            return Err(format!("NATS ping failed after {} attempts: {}", retry_count, e));
                        }
                    }
                }
            }
            Err(e) => {
                warn!("❌ NATS connection attempt {} failed: {}", attempt, e);
                if attempt == retry_count {
                    return Err(format!("Failed to connect to NATS after {} attempts: {}", retry_count, e));
                }
            }
        }
        
        // Exponential backoff: 1s, 2s, 3s, 4s, 5s
        let delay_secs = attempt;
        info!("⏱️  Waiting {}s before next attempt...", delay_secs);
        tokio::time::sleep(Duration::from_secs(delay_secs as u64)).await;
    }
    
    Err("Failed to connect to NATS".to_string())
}

fn main() {
    let args = Args::parse();
    let build_info = build_info::BuildInfo::get();

    // Handle NATS certificate contents if provided (Edgegap workaround)
    if let Some(ref ca_contents) = args.ca_contents {
        handle_ca_contents(ca_contents);
    }

    // Test NATS connection with retry logic before starting server (bevygap feature only)
    #[cfg(feature = "bevygap")]
    {
        // Only test NATS connection if we have environment variables suggesting we need NATS
        if env::var("NATS_HOST").is_ok() || env::var("NATS_USER").is_ok() {
            info!("🔌 NATS environment variables detected, testing connection...");
            
            // Create a minimal Tokio runtime for the async NATS test
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build() 
            {
                Ok(rt) => rt,
                Err(e) => {
                    error!("❌ Failed to create Tokio runtime for NATS testing: {}", e);
                    std::process::exit(1);
                }
            };

            match rt.block_on(test_nats_connection_with_retry(args.nats_retry_count)) {
                Ok(_) => {
                    info!("✅ NATS connection test passed, proceeding with server startup");
                }
                Err(e) => {
                    error!("❌ Failed to connect to NATS: {}", e);
                    error!("❌ Server startup aborted due to NATS connection failure");
                    std::process::exit(1);
                }
            }
        } else {
            info!("🔌 No NATS environment variables detected, skipping NATS connection test");
        }
    }

    // Generate certificate digest using the same approach as bevygap-spaceships
    let cert_digest = generate_certificate_digest();

    // Display the logo at startup

    println!(
        r#"


    ╔══════════════════════════════════════════════════════════════╗
    ║                                                              ║
    ║     ██╗   ██╗ ██████╗ ██╗██████╗                             ║
    ║     ██║   ██║██╔═══██╗██║██╔══██╗                            ║
    ║     ██║   ██║██║   ██║██║██║  ██║                            ║
    ║     ╚██╗ ██╔╝██║   ██║██║██║  ██║                            ║
    ║      ╚████╔╝ ╚██████╔╝██║██████╔╝                            ║
    ║       ╚═══╝   ╚═════╝ ╚═╝╚═════╝                             ║
    ║                                                              ║
    ║     ██╗      ██████╗  ██████╗ ██████╗                        ║
    ║     ██║     ██╔═══██╗██╔═══██╗██╔══██╗                       ║
    ║     ██║     ██║   ██║██║   ██║██████╔╝                       ║
    ║     ██║     ██║   ██║██║   ██║██╔═══╝                        ║
    ║     ███████╗╚██████╔╝╚██████╔╝██║                            ║
    ║     ╚══════╝ ╚═════╝  ╚═════╝ ╚═╝                            ║
    ║                                                              ║
    ║      ██████╗ ██╗   ██╗███████╗███████╗████████╗              ║
    ║     ██╔═══██╗██║   ██║██╔════╝██╔════╝╚══██╔══╝              ║
    ║     ██║   ██║██║   ██║█████╗  ███████╗   ██║                 ║
    ║     ██║▄▄ ██║██║   ██║██╔══╝  ╚════██║   ██║                 ║
    ║     ╚██████╔╝╚██████╔╝███████╗███████║   ██║                 ║
    ║      ╚══▀▀═╝  ╚═════╝ ╚══════╝╚══════╝   ╚═╝                 ║
    ║                                                              ║
    ║                  🚀 Server Starting... 🚀                    ║
    ╚══════════════════════════════════════════════════════════════╝
    "#
    );
    info!("🎮 Simple Platformer Server starting...");
    info!("📡 Listening on {}:{}", args.host, args.port);
    info!("🚢 Transport port: {}", args.transport_port);
    info!("🔄 Transport type: {}", args.transport);
    info!("📋 {}", build_info.format_for_log());
    info!("🔧 Build Details:");
    info!("   Git SHA: {}", build_info.git_sha);
    info!("   Git Branch: {}", build_info.git_branch);
    info!("   Build Time: {}", build_info.build_timestamp);
    info!("   Target: {}", build_info.target_triple);
    info!("   Author: {}", build_info.git_commit_author);
    info!("   System: {}", build_info.system_info);

    // Log certificate digest information
    if let Some(ref digest) = cert_digest {
        info!("🔐 Certificate digest generated: {}", &digest[..16]);
        info!("🔐 Digest available for WebTransport clients");
    } else {
        warn!("🔐 No certificate digest available - WebTransport may not work");
    }

    App::new().add_plugins(ServerPlugin::new(cert_digest)).run();
}

/// Generate certificate digest using the same approach as bevygap-spaceships
/// This creates a self-signed certificate and returns its SHA-256 digest
fn generate_certificate_digest() -> Option<String> {
    use sha2::{Digest, Sha256};

    // Try to get digest from environment variable first (for compatibility)
    if let Ok(digest) = env::var("LIGHTYEAR_CERTIFICATE_DIGEST") {
        if !digest.is_empty() {
            info!("🔐 Using certificate digest from LIGHTYEAR_CERTIFICATE_DIGEST");
            return Some(digest);
        }
    }

    // Get ARBITRIUM_PUBLIC_IP and SELF_SIGNED_SANS from environment (like bevygap-spaceships)
    let arbitrium_public_ip =
        env::var("ARBITRIUM_PUBLIC_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let self_signed_sans =
        env::var("SELF_SIGNED_SANS").unwrap_or_else(|_| format!("{}:5001", arbitrium_public_ip));

    info!(
        "🔐 Generating self-signed certificate with SANS: {}",
        self_signed_sans
    );

    // Create self-signed certificate (similar to bevygap-spaceships approach)
    match create_self_signed_cert(&self_signed_sans) {
        Ok(cert_der) => {
            // Generate SHA-256 digest
            let mut hasher = Sha256::new();
            hasher.update(&cert_der);
            let digest = hasher.finalize();
            let digest_hex = hex::encode(digest);

            info!("🔐 Generated certificate digest from self-signed cert");
            Some(digest_hex)
        }
        Err(e) => {
            warn!("🔐 Failed to generate self-signed certificate: {}", e);

            // Fallback: generate a deterministic digest based on server properties
            let mut hasher = Sha256::new();
            hasher.update(arbitrium_public_ip.as_bytes());
            hasher.update(self_signed_sans.as_bytes());

            // Include LIGHTYEAR_PRIVATE_KEY if available (like bevygap-spaceships)
            if let Ok(private_key) = env::var("LIGHTYEAR_PRIVATE_KEY") {
                hasher.update(private_key.as_bytes());
            }

            // Add build information for uniqueness
            hasher.update(env!("VERGEN_GIT_SHA").as_bytes());
            hasher.update(b"voidloop-quest-server-development");

            let digest = hasher.finalize();
            let digest_hex = hex::encode(digest);

            info!("🔐 Generated fallback certificate digest");
            Some(digest_hex)
        }
    }
}

/// Create a self-signed certificate (similar to bevygap-spaceships server::Identity::self_signed)
fn create_self_signed_cert(sans: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use rcgen::{Certificate, CertificateParams, SanType};

    let mut params = CertificateParams::new(vec![sans.to_string()]);

    // Add subject alternative names
    let san_parts: Vec<&str> = sans.split(',').collect();
    for san in san_parts {
        let san = san.trim();
        if san.parse::<std::net::IpAddr>().is_ok() {
            params
                .subject_alt_names
                .push(SanType::IpAddress(san.parse()?));
        } else {
            params
                .subject_alt_names
                .push(SanType::DnsName(san.to_string()));
        }
    }

    // Generate the certificate
    let cert = Certificate::from_params(params)?;
    let cert_der = cert.serialize_der()?;

    Ok(cert_der)
}

/// Handle CA certificate contents by writing them to a temporary file and setting NATS_CA env var
/// This is a workaround for Edgegap's 255-byte environment variable limit
fn handle_ca_contents(ca_contents: &str) {
    use std::fs;
    use std::io::Write;

    info!("🔐 Processing CA certificate contents...");

    // Create a temporary file for the certificate
    let temp_path = "/tmp/nats_ca.pem";

    match fs::File::create(temp_path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(ca_contents.as_bytes()) {
                warn!("🔐 Failed to write CA certificate to temporary file: {}", e);
                return;
            }

            info!("🔐 CA certificate written to: {}", temp_path);

            // Set the NATS_CA environment variable to point to the temporary file
            env::set_var("NATS_CA", temp_path);
            info!("🔐 NATS_CA environment variable set to: {}", temp_path);
        }
        Err(e) => {
            warn!(
                "🔐 Failed to create temporary file for CA certificate: {}",
                e
            );
        }
    }
}
