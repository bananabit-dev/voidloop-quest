use bevy::prelude::*;
use clap::Parser;
use server_plugin::ServerPlugin;
use std::env;

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
    
    // Generate or retrieve certificate digest
    let certificate_digest = get_certificate_digest();
    
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
    info!("🔐 Certificate digest: {}", certificate_digest);
    
    // Log build information
    log_build_info();

    App::new()
        .add_plugins(ServerPlugin::new(certificate_digest))
        .run();
}

/// Get certificate digest from environment or generate a fallback
fn get_certificate_digest() -> String {
    // First check if LIGHTYEAR_CERTIFICATE_DIGEST is set
    if let Ok(digest) = env::var("LIGHTYEAR_CERTIFICATE_DIGEST") {
        if !digest.is_empty() {
            info!("Using certificate digest from LIGHTYEAR_CERTIFICATE_DIGEST environment variable");
            return digest;
        }
    }
    
    // Fallback: generate a deterministic digest based on build info
    // This ensures consistent digest for the same build
    let build_hash = format!(
        "{}:{}:{}",
        env!("VERGEN_GIT_SHA"),
        env!("VERGEN_BUILD_TIMESTAMP"), 
        env!("VERGEN_CARGO_TARGET_TRIPLE")
    );
    
    // Create a simple SHA-256-like hash representation
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    build_hash.hash(&mut hasher);
    let hash = hasher.finish();
    
    // Format as a digest-like string
    let digest = format!("sha256:{:016x}", hash);
    info!("Generated certificate digest from build info: {}", digest);
    
    digest
}

/// Log build information from vergen
fn log_build_info() {
    info!("📦 Build info:");
    info!("  Git SHA: {}", env!("VERGEN_GIT_SHA"));
    info!("  Build timestamp: {}", env!("VERGEN_BUILD_TIMESTAMP"));
    info!("  Rust version: {}", env!("VERGEN_RUSTC_SEMVER"));
    info!("  Target triple: {}", env!("VERGEN_CARGO_TARGET_TRIPLE"));
}
