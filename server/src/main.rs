use bevy::prelude::*;
use clap::Parser;
use server_plugin::ServerPlugin;
use std::env;

mod build_info;
mod server_plugin;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 5001)]
    port: u16,

    /// Transport type (websocket or webtransport)
    #[arg(short, long, default_value = "websocket")]
    transport: String,
}

fn main() {
    let args = Args::parse();
    let build_info = build_info::BuildInfo::get();
    
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
    info!("📡 Listening on port {}", args.port);
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
    let arbitrium_public_ip = env::var("ARBITRIUM_PUBLIC_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let self_signed_sans = env::var("SELF_SIGNED_SANS").unwrap_or_else(|_| format!("{}:5001", arbitrium_public_ip));
    
    info!("🔐 Generating self-signed certificate with SANS: {}", self_signed_sans);
    
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
            params.subject_alt_names.push(SanType::IpAddress(san.parse()?));
        } else {
            params.subject_alt_names.push(SanType::DnsName(san.to_string()));
        }
    }
    
    // Generate the certificate
    let cert = Certificate::from_params(params)?;
    let cert_der = cert.serialize_der()?;
    
    Ok(cert_der)
}
