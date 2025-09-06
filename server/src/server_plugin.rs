// Simple room management that works locally for now
use bevy::prelude::*;
#[cfg(feature = "bevygap")]
use bevygap_server_plugin::prelude::BevygapServerPlugin;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;
use std::collections::HashMap;

use shared::{Platform, Player, PlayerActions, RoomInfo, SharedPlugin};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        // Minimal Bevy plugins for server
        app.add_plugins(
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
                std::time::Duration::from_secs_f32(1.0 / 60.0), // 60 FPS server tick rate
            )),
        );

        // Add input plugin for shared systems that need it
        app.add_plugins(InputManagerPlugin::<PlayerActions>::default());

        // Add mouse button input resource that the input manager expects
        app.init_resource::<bevy::input::ButtonInput<bevy::input::mouse::MouseButton>>();

        // Networking
        #[cfg(feature = "bevygap")]
        app.add_plugins(BevygapServerPlugin);

        // Shared game logic
        app.add_plugins(SharedPlugin);

        // Room management
        app.insert_resource(RoomRegistry::new());
        app.insert_resource(MatchmakingQueue::new());

        // Server-specific systems
        app.add_systems(Startup, setup_world);

        // Player management system - handles spawning/despawning players
        app.add_systems(Update, (handle_player_management, manage_room_lifecycle));

        // ==== CUSTOM SERVER SYSTEMS AREA - Add your server-specific logic here ====
        // Example: Game rules, scoring, AI, matchmaking logic, etc.
        // app.add_systems(Update, your_custom_server_system);
        // ==== END CUSTOM SERVER SYSTEMS AREA ====
    }
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
    // For now, we'll manually spawn a test player to verify the game works
    // In production, bevygap will handle player connections automatically
    _existing_players: Query<Entity, With<Player>>,
) {
    // Only spawn a test player if none exist and we're not using networking
    // This helps with local development
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

// ==== PLACEHOLDER FOR FUTURE NETWORKING FEATURES ====
// TODO: Add room message handling when lightyear API is fully integrated
// TODO: Add matchmaking queue processing
// ==== END PLACEHOLDER ====
