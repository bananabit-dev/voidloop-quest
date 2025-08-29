use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use shared::prelude::*;
use std::net::{Ipv4Addr, SocketAddr};

mod client_plugin;
pub(crate) mod screens;
use client_plugin::*;

fn main() {
    let mut app = App::new();

    // Display the logo at startup
    println!(r#"
    ╔══════════════════════════════════════════════════════════════╗
    ║                                                              ║
    ║     ██╗   ██╗ ██████╗ ██╗██████╗                            ║
    ║     ██║   ██║██╔═══██╗██║██╔══██╗                           ║
    ║     ██║   ██║██║   ██║██║██║  ██║                           ║
    ║     ╚██╗ ██╔╝██║   ██║██║██║  ██║                           ║
    ║      ╚████╔╝ ╚██████╔╝██║██████╔╝                           ║
    ║       ╚═══╝   ╚═════╝ ╚═╝╚═════╝                            ║
    ║                                                              ║
    ║     ██╗      ██████╗  ██████╗ ██████╗                       ║
    ║     ██║     ██╔═══██╗██╔═══██╗██╔══██╗                      ║
    ║     ██║     ██║   ██║██║   ██║██████╔╝                      ║
    ║     ██║     ██║   ██║██║   ██║██╔═══╝                       ║
    ║     ███████╗╚██████╔╝╚██████╔╝██║                           ║
    ║     ╚══════╝ ╚═════╝  ╚═════╝ ╚═╝                           ║
    ║                                                              ║
    ║      ██████╗ ██╗   ██╗███████╗███████╗████████╗             ║
    ║     ██╔═══██╗██║   ██║██╔════╝██╔════╝╚══██╔══╝             ║
    ║     ██║   ██║██║   ██║█████╗  ███████╗   ██║                ║
    ║     ██║▄▄ ██║██║   ██║██╔══╝  ╚════██║   ██║                ║
    ║     ╚██████╔╝╚██████╔╝███████╗███████║   ██║                ║
    ║      ╚══▀▀═╝  ╚═════╝ ╚══════╝╚══════╝   ╚═╝                ║
    ║                                                              ║
    ║                  🎮 Client Starting... 🎮                    ║
    ╚══════════════════════════════════════════════════════════════╝
    "#);

    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<LogPlugin>()
            .set(AssetPlugin {
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..default()
            }),
    );
    
    app.add_plugins(LogPlugin {
        level: Level::INFO,
        filter: "bevy_render=info,bevy_ecs=warn".to_string(),
        ..default()
    });

    info!("🎮 Void Loop Quest Client starting...");

    // Spawn client entity based on feature flags
    #[cfg(feature = "bevygap")]
    {
        info!("🔐 Using BevyGap for matchmaking and connection");
        // BevyGap will handle client spawning after matchmaking
    }
    
    #[cfg(not(feature = "bevygap"))]
    {
        info!("🔐 Direct connection mode (no matchmaking)");
        spawn_local_client(&mut app);
    }

    // Add game plugins
    app.add_plugins(BevygapSpaceshipsSharedPlugin);
    app.add_plugins(BevygapSpaceshipsClientPlugin);

    app.run();
}

#[cfg(not(feature = "bevygap"))]
fn spawn_local_client(app: &mut App) {
    use lightyear::netcode::NetcodeClient;
    use lightyear::webtransport::client::WebTransportClientIo;
    
    // Generate a random client ID
    #[cfg(target_arch = "wasm32")]
    let client_id: u64 = (web_sys::js_sys::Math::random() * u64::MAX as f64) as u64;
    #[cfg(not(target_arch = "wasm32"))]
    let client_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    info!("Client ID: {client_id}");
    
    let client_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0);
    let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), SERVER_PORT);
    
    // Read private key from environment or use dummy
    let private_key = parse_private_key_from_env().unwrap_or(DUMMY_PRIVATE_KEY);
    
    // Certificate digest for WebTransport
    #[cfg(target_family = "wasm")]
    let certificate_digest = get_certificate_digest();
    #[cfg(not(target_family = "wasm"))]
    let certificate_digest = String::new();
    
    info!("Connecting to server at {server_addr}");
    
    // Spawn client entity with all necessary components
    let client_entity = app.world_mut().spawn((
        Client::default(),
        Name::from("Client"),
        LocalAddr(client_addr),
        PeerAddr(server_addr),
        Link::new(None),
        ReplicationReceiver::default(),
        PredictionManager::default(),
        InterpolationManager::default(),
        NetcodeClient::new(
            Authentication::Manual {
                server_addr,
                client_id,
                private_key,
                protocol_id: PROTOCOL_ID,
            },
            NetcodeConfig::default(),
        ).expect("Failed to create NetcodeClient"),
        WebTransportClientIo { certificate_digest },
    )).id();
    
    info!("Created client entity: {client_entity:?}");
}

#[cfg(target_family = "wasm")]
fn get_certificate_digest() -> String {
    // Try to get from environment variable first
    if let Ok(digest) = std::env::var("LIGHTYEAR_CERTIFICATE_DIGEST") {
        if !digest.is_empty() {
            info!("Using certificate digest from env: {}", digest);
            return digest;
        }
    }
    
    // Try to get from URL hash
    let window = web_sys::window().expect("expected window");
    if let Ok(hash) = window.location().hash() {
        let digest = hash.replace('#', "");
        if digest.len() > 10 {
            info!("Using certificate digest from URL hash: {}", digest);
            return digest;
        }
    }
    
    // Try to get from window object
    if let Some(obj) = window.get("CERT_DIGEST") {
        if let Some(digest) = obj.as_string() {
            info!("Using certificate digest from window.CERT_DIGEST: {}", digest);
            return digest;
        }
    }
    
    warn!("No certificate digest found, connection may fail");
    String::new()
}

/// Parse private key from environment variable
fn parse_private_key_from_env() -> Option<[u8; 32]> {
    let Ok(key_str) = std::env::var("LIGHTYEAR_PRIVATE_KEY") else {
        return None;
    };
    
    let private_key: Vec<u8> = key_str
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect::<String>()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<u8>()
                .expect("Failed to parse number in private key")
        })
        .collect();

    if private_key.len() != 32 {
        panic!("Private key must contain exactly 32 numbers, got {}", private_key.len());
    }

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&private_key);
    Some(bytes)
}
