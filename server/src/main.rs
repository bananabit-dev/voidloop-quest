use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use shared::prelude::*;
use std::net::{Ipv4Addr, SocketAddr};

mod server_plugin;
use server_plugin::*;

// Private key for netcode authentication
// This should be read from ENV in production
pub const PRIVATE_KEY: [u8; 32] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
];

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
    ║                  🚀 Server Starting... 🚀                    ║
    ╚══════════════════════════════════════════════════════════════╝
    "#);

    #[cfg(feature = "gui")]
    app.add_plugins(
        DefaultPlugins
            .build()
            // logger added with custom config below
            .disable::<LogPlugin>(),
    );

    #[cfg(not(feature = "gui"))]
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
    ));

    app.add_plugins(LogPlugin {
        level: Level::INFO,
        filter: "bevy_render=info,bevy_ecs=warn".to_string(),
        ..default()
    });

    info!("🚀 Void Loop Quest Server starting...");
    info!("⭐️ Build time: {}", env!("VERGEN_BUILD_TIMESTAMP"));
    info!("⭐️ Git desc: {}", env!("VERGEN_GIT_DESCRIBE"));
    info!("⭐️ Git sha: {}", env!("VERGEN_GIT_SHA"));
    info!("⭐️ Git commit @ {}", env!("VERGEN_GIT_COMMIT_TIMESTAMP"));

    // Read private key from environment or use default
    let private_key = parse_private_key_from_env().unwrap_or_else(|| {
        warn!("LIGHTYEAR_PRIVATE_KEY not set, using dummy key");
        PRIVATE_KEY
    });

    // Spawn the server entity with all necessary components
    let server_entity = app.world_mut().spawn((
        Server::default(),
        Name::from("Server"),
    )).id();

    // Configure server based on features
    #[cfg(feature = "bevygap")]
    {
        info!("🔐 Configuring server for Edgegap deployment with WebTransport");
        configure_webtransport_server(&mut app, server_entity, private_key);
    }

    #[cfg(not(feature = "bevygap"))]
    {
        info!("🔐 Configuring local server with WebTransport");
        configure_local_server(&mut app, server_entity, private_key);
    }

    // Add shared and server plugins
    app.add_plugins(BevygapSpaceshipsSharedPlugin);
    app.add_plugins(BevygapSpaceshipsServerPlugin);

    // Start the server
    app.add_systems(Startup, start_server);

    app.run();
}

fn configure_webtransport_server(app: &mut App, server_entity: Entity, private_key: [u8; 32]) {
    let mut sans = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];

    // Check if running on Edgegap
    if let Ok(public_ip) = std::env::var("ARBITRIUM_PUBLIC_IP") {
        info!("🔐 SAN += ARBITRIUM_PUBLIC_IP: {}", public_ip);
        sans.push(public_ip);
        sans.push("*.pr.edgegap.net".to_string());
    }

    // Additional SANs from environment
    if let Ok(san) = std::env::var("SELF_SIGNED_SANS") {
        info!("🔐 SAN += SELF_SIGNED_SANS: {}", san);
        sans.extend(san.split(',').map(|s| s.to_string()));
    }

    info!("🔐 Creating self-signed certificate with SANs: {:?}", sans);
    
    // Generate self-signed certificate
    let identity = Identity::self_signed(sans)
        .expect("Failed to create self-signed certificate");
    
    let digest = identity.certificate_chain().as_slice()[0].hash();
    info!("🔐 Certificate digest: {}", digest);

    let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), SERVER_PORT);
    info!("📡 Listening on {}", server_addr);

    // Add components to server entity
    app.world_mut().entity_mut(server_entity).insert((
        LocalAddr(server_addr),
        WebTransportServerIo { certificate: identity },
        NetcodeServer::new(NetcodeConfig {
            protocol_id: PROTOCOL_ID,
            private_key,
            ..Default::default()
        }),
        Link::new(None),
    ));
}

fn configure_local_server(app: &mut App, server_entity: Entity, private_key: [u8; 32]) {
    let sans = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];
    
    info!("🔐 Creating self-signed certificate for local development");
    
    let identity = Identity::self_signed(sans)
        .expect("Failed to create self-signed certificate");
    
    let digest = identity.certificate_chain().as_slice()[0].hash();
    info!("🔐 Certificate digest: {}", digest);

    let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), SERVER_PORT);
    info!("📡 Listening on {}", server_addr);

    // Add components to server entity
    app.world_mut().entity_mut(server_entity).insert((
        LocalAddr(server_addr),
        WebTransportServerIo { certificate: identity },
        NetcodeServer::new(NetcodeConfig {
            protocol_id: PROTOCOL_ID,
            private_key,
            ..Default::default()
        }),
        Link::new(None),
    ));
}

fn start_server(mut commands: Commands, server: Query<Entity, With<Server>>) {
    if let Ok(server_entity) = server.single() {
        info!("🚀 Starting server...");
        commands.trigger_targets(Start, server_entity);
    }
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
