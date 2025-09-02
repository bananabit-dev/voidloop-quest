use bevy::prelude::*;
use bevygap_server_plugin::prelude::BevygapServerPlugin;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;

use shared::{Player, PlayerActions, PlayerColor, PlayerTransform, Platform, SharedPlugin};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        // Minimal Bevy plugins for server
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_secs_f32(1.0 / 60.0), // 60 FPS server tick rate
        ));
        
        // Input plugin
        app.add_plugins(InputManagerPlugin::<PlayerActions>::default());
        
        // Networking
        app.add_plugins(BevygapServerPlugin);
        
        // Shared game logic
        app.add_plugins(SharedPlugin);
        
        // Room management
        app.insert_resource(RoomManager::new());
        
        // Server-specific systems
        app.add_systems(Startup, setup_world);
        
        // Player management system - handles spawning/despawning players  
        app.add_systems(Update, (
            handle_player_management,
            manage_room_lifecycle,
        ));
        
        // ==== CUSTOM SERVER SYSTEMS AREA - Add your server-specific logic here ====
        // Example: Game rules, scoring, AI, matchmaking logic, etc.
        // app.add_systems(Update, your_custom_server_system);
        // ==== END CUSTOM SERVER SYSTEMS AREA ====
    }
}

fn setup_world(mut commands: Commands) {
    info!("Setting up game world...");
    
    // Spawn platforms (these will be replicated to clients)
    let platform_positions = vec![
        Vec3::new(-200.0, -100.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(200.0, -50.0, 0.0),
        Vec3::new(-300.0, 50.0, 0.0),
        Vec3::new(300.0, 100.0, 0.0),
    ];
    
    for pos in platform_positions {
        commands.spawn((
            Platform,
            Transform::from_translation(pos),
            Replicate::default(),
        ));
    }
    
    info!("World setup complete with {} platforms", 5);
}

// Player management system that handles room logic
fn handle_player_management(
    mut commands: Commands,
    // For now, we'll manually spawn a test player to verify the game works
    // In production, bevygap will handle player connections automatically
    existing_players: Query<Entity, With<Player>>,
) {
    // Spawn a test player if none exist (for testing without actual networking)
    if existing_players.is_empty() {
        info!("No players found - spawning test player for local game");
        
        let spawn_pos = Vec3::new(0.0, 100.0, 0.0);
        let color = Color::srgb(0.5, 0.8, 1.0);
        
        commands.spawn((
            Player::default(),
            PlayerTransform {
                translation: spawn_pos,
            },
            PlayerColor { color },
            InputMap::<PlayerActions>::default(),
            ActionState::<PlayerActions>::default(),
            Replicate::default(),
        ));
        
        info!("Spawned test player at position {:?}", spawn_pos);
    }
}

// Room lifecycle management - handles auto-cleanup and game state
fn manage_room_lifecycle(
    mut room_manager: ResMut<RoomManager>,
    players: Query<Entity, With<Player>>,
    time: Res<Time>,
) {
    let current_player_count = players.iter().count() as u32;
    
    // Update player count
    if room_manager.player_count != current_player_count {
        let old_count = room_manager.player_count;
        room_manager.player_count = current_player_count;
        
        if current_player_count > old_count {
            info!("Player joined room '{}'. Players: {}/{}", 
                  room_manager.room_id, current_player_count, room_manager.max_players);
        } else if current_player_count < old_count {
            info!("Player left room '{}'. Players: {}/{}", 
                  room_manager.room_id, current_player_count, room_manager.max_players);
        }
        
        // Check if game should start
        if room_manager.should_start_game() && old_count < room_manager.min_players {
            info!("🚀 Room '{}' has minimum players ({}) - game can start!", 
                  room_manager.room_id, room_manager.min_players);
        }
    }
    
    // Auto-cleanup: If room is empty for too long, it could be cleaned up
    // For now, we'll just log it since bevygap handles server lifecycle
    if room_manager.is_empty() {
        if room_manager.room_created_time.is_none() {
            room_manager.room_created_time = Some(time.elapsed_secs_f64());
            info!("Room '{}' is now empty - starting cleanup timer", room_manager.room_id);
        } else if let Some(empty_since) = room_manager.room_created_time {
            let empty_duration = time.elapsed_secs_f64() - empty_since;
            if empty_duration > 10.0 { // 10 seconds cleanup time
                info!("Room '{}' has been empty for {:.1}s - would cleanup in production", 
                      room_manager.room_id, empty_duration);
            }
        }
    } else {
        // Reset cleanup timer if players are present
        if room_manager.room_created_time.is_some() {
            room_manager.room_created_time = None;
        }
    }
}

// Room management resource - tracks active rooms and player counts
#[derive(Resource, Default)]
pub struct RoomManager {
    pub room_id: String,
    pub player_count: u32,
    pub max_players: u32,
    pub min_players: u32,
    pub room_created_time: Option<f64>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            room_id: "default-room".to_string(),
            player_count: 0,
            max_players: 4,  // As requested in problem statement
            min_players: 1,  // As requested in problem statement  
            room_created_time: None,
        }
    }
    
    pub fn can_add_player(&self) -> bool {
        self.player_count < self.max_players
    }
    
    pub fn should_start_game(&self) -> bool {
        self.player_count >= self.min_players
    }
    
    pub fn is_empty(&self) -> bool {
        self.player_count == 0
    }
}

// ==== CUSTOM SERVER LOGIC AREA - Add your game rules and server logic here ====
// Example: Scoring system, match management, AI opponents, etc.
// 
// fn my_custom_server_logic(
//     // your queries and resources
// ) {
//     // your server logic
// }
// ==== END CUSTOM SERVER LOGIC AREA ====
