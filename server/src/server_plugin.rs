// Simple room management that works locally for now
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use std::collections::HashMap;
use std::env;

#[cfg(feature = "bevygap")]
use bevygap_server_plugin::prelude::*;
#[cfg(feature = "bevygap")]
use lightyear::prelude::{server, *};
#[cfg(feature = "bevygap")]
use lightyear::prelude::server::{NetcodeServer, NetcodeConfig};

use crate::build_info::BuildInfo;
use shared::{Platform, Player, PlayerActions, RoomInfo, SharedPlugin};

// Constants for Lightyear private key handling
const DUMMY_PRIVATE_KEY: [u8; 32] = [0; 32]; // All zeros for local development

/// Parse the LIGHTYEAR_PRIVATE_KEY environment variable into a 32-byte array
/// Supports formats like:
/// - "[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]"
/// - "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32"
fn read_lightyear_private_key_from_env() -> Option<[u8; 32]> {
    let key_str = std::env::var("LIGHTYEAR_PRIVATE_KEY").ok()?;
    
    // Remove brackets and whitespace
    let cleaned = key_str.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .replace(' ', "");
    
    // Split by comma and parse each byte
    let bytes: Result<Vec<u8>, _> = cleaned
        .split(',')
        .map(|s| s.trim().parse::<u8>())
        .collect();
    
    match bytes {
        Ok(byte_vec) if byte_vec.len() == 32 => {
            let mut key = [0u8; 32];
            key.copy_from_slice(&byte_vec);
            info!("🔐 Successfully parsed LIGHTYEAR_PRIVATE_KEY from environment");
            Some(key)
        }
        Ok(byte_vec) => {
            warn!(
                "🔐 LIGHTYEAR_PRIVATE_KEY has wrong length: expected 32 bytes, got {}",
                byte_vec.len()
            );
            None
        }
        Err(e) => {
            warn!("🔐 Failed to parse LIGHTYEAR_PRIVATE_KEY: {}", e);
            None
        }
    }
}

pub struct ServerPlugin {
    pub cert_digest: Option<String>,
}

impl ServerPlugin {
    pub fn new(cert_digest: Option<String>) -> Self {
        Self { cert_digest }
    }
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        // Minimal Bevy plugins for server
        app.add_plugins(
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
                std::time::Duration::from_secs_f32(1.0 / 60.0), // 60 FPS server tick rate
            )),
        );

        app.add_plugins(LogPlugin {
            level: Level::INFO, // global default if RUST_LOG not set
            filter: env::var("RUST_LOG").unwrap_or_else(|_| {
                // Fallback filter string (you can tune this)
                "info,bevygap_server_plugin=info,lightyear=info,server=info".to_string()
            }),
            ..default()
        });

        // Add input plugin for shared systems that need it
        app.add_plugins(InputManagerPlugin::<PlayerActions>::default());

        // Add input resources that the input manager expects (minimal setup for headless server)
        app.init_resource::<bevy::input::ButtonInput<bevy::input::mouse::MouseButton>>();
        app.init_resource::<bevy::input::ButtonInput<bevy::input::keyboard::KeyCode>>();
        app.init_resource::<bevy::input::mouse::AccumulatedMouseMotion>();
        app.init_resource::<bevy::input::mouse::AccumulatedMouseScroll>();

        // Register Lightyear protocol (components, channels, inputs)
        app.add_plugins(shared::protocol());

        // Networking (Lightyear server) and Edgegap integration
        #[cfg(feature = "bevygap")]
        {
            // Configure and add Lightyear server plugins for networking
            app.add_plugins(server::ServerPlugins {
                tick_duration: std::time::Duration::from_secs_f32(1.0 / 60.0),
            });

            // Configure the server with private key and protocol ID
            app.add_systems(Startup, setup_netcode_server);

            // Add Bevygap integration (NATS, metadata)
            app.add_plugins(BevygapServerPlugin);
        }

        // Shared game logic
        app.add_plugins(SharedPlugin);

        // Room management
        app.insert_resource(RoomRegistry::new());
        app.insert_resource(MatchmakingQueue::new());

        // Build metadata for diagnostics
        app.insert_resource(BuildInfo::get());

        app.insert_resource(ServerMetadata::new(self.cert_digest.clone()));

        // Server-specific systems
        app.add_systems(Startup, (setup_world, setup_server_metadata));

        app.add_systems(
            Update,
            (
                handle_player_management,
                manage_room_lifecycle,
                log_server_status,
            ),
        );
    }
}

#[cfg(feature = "bevygap")]
fn setup_netcode_server(mut commands: Commands) {
    // Protocol ID and private key from env (see README/setup.sh)
    let protocol_id: u64 = std::env::var("LIGHTYEAR_PROTOCOL_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80085);

    let key = read_lightyear_private_key_from_env().unwrap_or_else(|| {
        warn!("LIGHTYEAR_PRIVATE_KEY not set, using dummy key");
        DUMMY_PRIVATE_KEY
    });

    info!("🔐 Setting up Lightyear server with protocol_id: {}", protocol_id);
    if std::env::var("LIGHTYEAR_PRIVATE_KEY").is_ok() {
        info!("🔐 Using LIGHTYEAR_PRIVATE_KEY from environment");
    } else {
        warn!("🔐 Using dummy private key for development (insecure!)");
    }

    let netcode_config = NetcodeConfig::default()
        .with_protocol_id(protocol_id)
        .with_key(key);

    // Spawn the server with netcode configuration
    commands.spawn(NetcodeServer::new(netcode_config));
}

fn setup_world(mut commands: Commands) {
    info!("Setting up game world...");

    // Spawn platforms (these will be replicated to clients in networked mode)
    let platform_positions = vec![
        Vec3::new(-200.0, -100.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(200.0, -50.0, 0.0),
        Vec3::new(-300.0, 50.0, 0.0),
        Vec3::new(300.0, 100.0, 0.0),
    ];

    for pos in platform_positions {
        #[cfg(feature = "bevygap")]
        {
            commands.spawn((
                Platform,
                Transform::from_translation(pos),
                Replicate::default(),
            ));
        }
        #[cfg(not(feature = "bevygap"))]
        {
            commands.spawn((Platform, Transform::from_translation(pos)));
        }
    }

    info!("World setup complete with {} platforms", 5);
}

// Player management system that handles room logic
fn handle_player_management(
    _commands: Commands,
    _existing_players: Query<Entity, With<Player>>,
) {
    // Only spawn a test player if none exist and we're not using networking
    #[cfg(not(feature = "bevygap"))]
    {
        // Note: Player spawning is now handled client-side for better local development experience
        // Server-side player spawning will be re-enabled when proper networking is integrated
    }
}

// Room lifecycle management - handles auto-cleanup and game state
fn manage_room_lifecycle(
    mut room_registry: ResMut<RoomRegistry>,
    players: Query<Entity, With<Player>>,
    time: Res<Time>,
) {
    let current_player_count = players.iter().count() as u32;

    // Update player count for all rooms
    let mut rooms_to_remove = Vec::new();
    let room_ids: Vec<String> = room_registry.rooms.keys().cloned().collect();

    for room_id in room_ids {
        if let Some(room) = room_registry.rooms.get_mut(&room_id) {
            let old_count = room.current_players;
            room.current_players = current_player_count;

            if room.current_players > old_count {
                info!(
                    "Player joined room '{}'. Players: {}/{}",
                    room.room_id, room.current_players, room.max_players
                );
            } else if room.current_players < old_count {
                info!(
                    "Player left room '{}'. Players: {}/{}",
                    room.room_id, room.current_players, room.max_players
                );
            }

            // Check if game should start
            if room.current_players >= 1 && old_count < 1 {
                info!(
                    "🚀 Room '{}' has minimum players ({}) - game can start!",
                    room.room_id, 1
                );
            }

            // Auto-cleanup empty rooms after 30 seconds
            if room.current_players == 0 {
                if room.created_time.is_none() {
                    room.created_time = Some(time.elapsed_secs_f64());
                    info!("Room '{}' is now empty - starting cleanup timer", room_id);
                } else if let Some(empty_since) = room.created_time {
                    let empty_duration = time.elapsed_secs_f64() - empty_since;
                    if empty_duration > 30.0 {
                        // 30 seconds cleanup time
                        info!(
                            "Room '{}' has been empty for {:.1}s - cleaning up",
                            room_id, empty_duration
                        );
                        rooms_to_remove.push(room_id.clone());
                    }
                }
            } else {
                // Reset cleanup timer if players are present
                room.created_time = None;
            }
        }
    }

    // Remove empty rooms
    for room_id in rooms_to_remove {
        room_registry.rooms.remove(&room_id);
        info!("Removed empty room: {}", room_id);
    }
}

// Server metadata resource - stores server information for diagnostics and client verification
#[derive(Resource, Debug, Clone)]
pub struct ServerMetadata {
    pub certificate_digest: Option<String>,
    pub fqdn: Option<String>,
    pub build_info: BuildInfo,
    pub startup_time: f64,
}

impl ServerMetadata {
    pub fn new(cert_digest: Option<String>) -> Self {
        Self {
            certificate_digest: cert_digest,
            fqdn: env::var("SERVER_FQDN").ok(),
            build_info: BuildInfo::get(),
            startup_time: 0.0,
        }
    }

    /// Get metadata as a formatted string for logging/debugging
    #[allow(dead_code)]
    pub fn to_debug_string(&self) -> String {
        format!(
            "ServerMetadata {{ git_sha: {}, build_time: {}, cert_digest: {}, fqdn: {}, uptime: {:.1}s }}",
            self.build_info.git_sha,
            self.build_info.build_timestamp,
            self.certificate_digest.as_deref().map(|d| &d[..16]).unwrap_or("None"),
            self.fqdn.as_deref().unwrap_or("None"),
            self.startup_time
        )
    }

    /// Get the certificate digest for API responses or client verification
    pub fn get_certificate_digest(&self) -> Option<&str> {
        self.certificate_digest.as_deref()
    }

    /// Check if the server has a valid certificate digest for secure connections
    pub fn has_certificate_digest(&self) -> bool {
        self.certificate_digest.is_some()
    }

    /// Get server information formatted for external APIs or diagnostics
    pub fn to_api_response(&self) -> String {
        serde_json::json!({
            "server": {
                "build_info": {
                    "git_sha": self.build_info.git_sha,
                    "git_branch": self.build_info.git_branch,
                    "build_timestamp": self.build_info.build_timestamp,
                    "rustc_version": self.build_info.rustc_version,
                    "target_triple": self.build_info.target_triple
                },
                "certificate_digest": self.certificate_digest,
                "fqdn": self.fqdn,
                "startup_time": self.startup_time,
                "has_certificate": self.has_certificate_digest()
            }
        })
        .to_string()
    }
}

// Initial setup system for server metadata
fn setup_server_metadata(mut metadata: ResMut<ServerMetadata>, time: Res<Time>) {
    metadata.startup_time = time.elapsed_secs_f64();

    info!("🔧 Server Metadata Initialized:");
    info!("  📋 Git SHA: {}", metadata.build_info.git_sha);
    info!("  🌳 Git Branch: {}", metadata.build_info.git_branch);
    info!("  ⏰ Build Time: {}", metadata.build_info.build_timestamp);
    info!("  🦀 Rust Version: {}", metadata.build_info.rustc_version);
    info!("  🎯 Target: {}", metadata.build_info.target_triple);

    if let Some(ref digest) = metadata.certificate_digest {
        info!("  🔐 Certificate Digest: {}...", &digest[..16]);
        info!("  📄 Digest available for WebTransport clients and API responses");
    } else {
        warn!("  🔐 Certificate Digest: Not available - WebTransport may not work");
        warn!(
            "  💡 Consider setting ARBITRIUM_PUBLIC_IP, SELF_SIGNED_SANS, or LIGHTYEAR_CERTIFICATE_DIGEST"
        );
    }

    if let Some(ref fqdn) = metadata.fqdn {
        info!("  🌐 Server FQDN: {}", fqdn);
    } else {
        info!("  🌐 Server FQDN: Not configured");
    }

    info!("  🚀 Startup Time: {:.3}s", metadata.startup_time);
}

/// System to periodically log server status with build information for diagnostics
fn log_server_status(
    time: Res<Time>,
    metadata: Res<ServerMetadata>,
    room_registry: Res<RoomRegistry>,
    mut last_log: Local<f32>,
) {
    let current_time = time.elapsed_secs();

    // Log server status every 5 minutes (300 seconds)
    if current_time - *last_log >= 300.0 {
        *last_log = current_time;

        info!("📊 Server Status Report:");
        info!("   Uptime: {:.1} minutes", current_time / 60.0);
        info!("   Active Rooms: {}", room_registry.rooms.len());
        info!("   Build: {}", metadata.build_info.format_for_log());
        info!(
            "   Git SHA: {} ({})",
            metadata.build_info.git_sha, metadata.build_info.git_branch
        );

        if let Some(digest) = metadata.get_certificate_digest() {
            info!(
                "   Certificate Digest: {}... (available for WebTransport)",
                &digest[..16]
            );
        } else {
            warn!("   Certificate Digest: Not available (WebTransport may not work)");
        }

        // Log room details if any exist
        if !room_registry.rooms.is_empty() {
            info!("   Room Details:");
            for (room_id, room_data) in &room_registry.rooms {
                info!("     Room {}: {} players", room_id, room_data.current_players);
            }
        }

        debug!("📋 Full server metadata: {}", metadata.to_debug_string());
    }
}

// Room management resource - tracks active rooms and player counts
#[derive(Resource, Default)]
pub struct RoomRegistry {
    pub rooms: HashMap<String, RoomData>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RoomData {
    pub room_id: String,
    pub host_name: String,
    pub game_mode: String,
    pub current_players: u32,
    pub max_players: u32,
    pub player_names: Vec<String>,
    pub created_time: Option<f64>,
}

#[derive(Resource, Default)]
#[allow(dead_code)]
pub struct MatchmakingQueue {
    pub queue: HashMap<String, Vec<MatchmakingPlayer>>, // game_mode -> players
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MatchmakingPlayer {
    pub player_id: String,
    pub join_time: f64,
}

impl RoomRegistry {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn create_room(
        &mut self,
        room_id: String,
        host_name: String,
        game_mode: String,
    ) -> RoomData {
        let room_data = RoomData {
            room_id: room_id.clone(),
            host_name,
            game_mode,
            current_players: 1,
            max_players: 4,
            player_names: Vec::new(),
            created_time: None,
        };
        self.rooms.insert(room_id.clone(), room_data.clone());
        room_data
    }

    #[allow(dead_code)]
    pub fn get_room_list(&self) -> Vec<RoomInfo> {
        self.rooms
            .values()
            .map(|room| RoomInfo {
                room_id: room.room_id.clone(),
                current_players: room.current_players,
                max_players: room.max_players,
                host_name: room.host_name.clone(),
                game_mode: room.game_mode.clone(),
            })
            .collect()
    }
}

impl MatchmakingQueue {
    pub fn new() -> Self {
        Self {
            queue: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_player(&mut self, game_mode: String, player_id: String, join_time: f64) {
        let queue = self.queue.entry(game_mode).or_default();
        queue.push(MatchmakingPlayer {
            player_id,
            join_time,
        });
    }

    #[allow(dead_code)]
    pub fn try_create_match(&mut self, game_mode: &str) -> Option<Vec<MatchmakingPlayer>> {
        if let Some(queue) = self.queue.get_mut(game_mode) {
            if queue.len() >= 4 {
                // Take first 4 players for a match
                let matched_players: Vec<_> = queue.drain(0..4).collect();
                return Some(matched_players);
            }
        }
        None
    }
}
