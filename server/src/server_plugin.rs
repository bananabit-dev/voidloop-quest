use bevy::prelude::*;
use bevygap_server_plugin::prelude::BevygapServerPlugin;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;
use std::collections::HashMap;

use shared::{Player, PlayerActions, PlayerColor, PlayerTransform, Platform, SharedPlugin, RoomMessage, RoomInfo, Channel1};

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
        app.insert_resource(RoomRegistry::new());
        app.insert_resource(MatchmakingQueue::new());
        
        // Server-specific systems
        app.add_systems(Startup, setup_world);
        
        // Player management system - handles spawning/despawning players  
        app.add_systems(Update, (
            handle_player_management,
            handle_room_messages,
            handle_matchmaking,
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
    mut room_registry: ResMut<RoomRegistry>,
    players: Query<Entity, With<Player>>,
    time: Res<Time>,
) {
    let current_player_count = players.iter().count() as u32;
    
    // Update player count for all rooms
    for room in room_registry.rooms.values_mut() {
        let old_count = room.current_players;
        room.current_players = current_player_count;
        
        if room.current_players > old_count {
            info!("Player joined room '{}'. Players: {}/{}", 
                  room.room_id, room.current_players, room.max_players);
        } else if room.current_players < old_count {
            info!("Player left room '{}'. Players: {}/{}", 
                  room.room_id, room.current_players, room.max_players);
        }
        
        // Check if game should start
        if room.current_players >= 1 && old_count < 1 {
            info!("🚀 Room '{}' has minimum players ({}) - game can start!", 
                  room.room_id, 1);
        }
    }
    
    // Auto-cleanup empty rooms after 30 seconds
    let mut rooms_to_remove = Vec::new();
    for (room_id, room) in &room_registry.rooms {
        if room.current_players == 0 {
            if room.created_time.is_none() {
                room_registry.rooms.get_mut(room_id).unwrap().created_time = Some(time.elapsed_secs_f64());
                info!("Room '{}' is now empty - starting cleanup timer", room_id);
            } else if let Some(empty_since) = room.created_time {
                let empty_duration = time.elapsed_secs_f64() - empty_since;
                if empty_duration > 30.0 { // 30 seconds cleanup time
                    info!("Room '{}' has been empty for {:.1}s - cleaning up", room_id, empty_duration);
                    rooms_to_remove.push(room_id.clone());
                }
            }
        } else {
            // Reset cleanup timer if players are present
            if room.created_time.is_some() {
                room_registry.rooms.get_mut(room_id).unwrap().created_time = None;
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
pub struct MatchmakingQueue {
    pub queue: HashMap<String, Vec<MatchmakingPlayer>>, // game_mode -> players
}

#[derive(Clone, Debug)]
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
    
    pub fn create_room(&mut self, room_id: String, host_name: String, game_mode: String) -> RoomData {
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
    
    pub fn get_room_list(&self) -> Vec<RoomInfo> {
        self.rooms.values().map(|room| RoomInfo {
            room_id: room.room_id.clone(),
            current_players: room.current_players,
            max_players: room.max_players,
            host_name: room.host_name.clone(),
            game_mode: room.game_mode.clone(),
        }).collect()
    }
}

impl MatchmakingQueue {
    pub fn new() -> Self {
        Self {
            queue: HashMap::new(),
        }
    }
    
    pub fn add_player(&mut self, game_mode: String, player_id: String, join_time: f64) {
        let queue = self.queue.entry(game_mode).or_insert_with(Vec::new);
        queue.push(MatchmakingPlayer { player_id, join_time });
    }
    
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

// Handle room management messages from clients
fn handle_room_messages(
    mut room_registry: ResMut<RoomRegistry>,
    mut server: ResMut<Server>,
    mut events: EventReader<MessageEvent<RoomMessage>>,
) {
    for event in events.read() {
        let client_id = event.context();
        let message = &event.message;
        
        match message {
            RoomMessage::CreateRoom { room_id, host_name, game_mode } => {
                info!("Creating room {} for host {}", room_id, host_name);
                let room_data = room_registry.create_room(room_id.clone(), host_name.clone(), game_mode.clone());
                
                let room_info = RoomInfo {
                    room_id: room_data.room_id,
                    current_players: room_data.current_players,
                    max_players: room_data.max_players,
                    host_name: room_data.host_name,
                    game_mode: room_data.game_mode,
                };
                
                // Send confirmation back to client
                let _ = server.send_message::<RoomMessage, Channel1>(
                    *client_id,
                    RoomMessage::RoomCreated { room_info }
                );
            },
            
            RoomMessage::JoinRoom { room_id, player_name } => {
                if let Some(room) = room_registry.rooms.get_mut(room_id) {
                    if room.current_players < room.max_players {
                        room.current_players += 1;
                        room.player_names.push(player_name.clone());
                        
                        let room_info = RoomInfo {
                            room_id: room.room_id.clone(),
                            current_players: room.current_players,
                            max_players: room.max_players,
                            host_name: room.host_name.clone(),
                            game_mode: room.game_mode.clone(),
                        };
                        
                        // Send confirmation to joiner
                        let _ = server.send_message::<RoomMessage, Channel1>(
                            *client_id,
                            RoomMessage::RoomJoined { room_info: room_info.clone() }
                        );
                        
                        // Broadcast player joined to all clients in room
                        let _ = server.broadcast_message::<RoomMessage, Channel1>(
                            RoomMessage::PlayerJoined { 
                                room_id: room_id.clone(), 
                                player_name: player_name.clone(), 
                                player_count: room.current_players 
                            }
                        );
                        
                        info!("Player {} joined room {}. Players: {}/{}", 
                              player_name, room_id, room.current_players, room.max_players);
                    } else {
                        let _ = server.send_message::<RoomMessage, Channel1>(
                            *client_id,
                            RoomMessage::RoomError { message: "Room is full".to_string() }
                        );
                    }
                } else {
                    let _ = server.send_message::<RoomMessage, Channel1>(
                        *client_id,
                        RoomMessage::RoomError { message: "Room not found".to_string() }
                    );
                }
            },
            
            RoomMessage::LeaveRoom { room_id, player_name } => {
                if let Some(room) = room_registry.rooms.get_mut(room_id) {
                    if let Some(pos) = room.player_names.iter().position(|x| x == player_name) {
                        room.player_names.remove(pos);
                        room.current_players = room.current_players.saturating_sub(1);
                        
                        // Send confirmation to leaver
                        let _ = server.send_message::<RoomMessage, Channel1>(
                            *client_id,
                            RoomMessage::RoomLeft { room_id: room_id.clone() }
                        );
                        
                        // Broadcast player left to all clients
                        let _ = server.broadcast_message::<RoomMessage, Channel1>(
                            RoomMessage::PlayerLeft { 
                                room_id: room_id.clone(), 
                                player_name: player_name.clone(), 
                                player_count: room.current_players 
                            }
                        );
                        
                        info!("Player {} left room {}. Players: {}/{}", 
                              player_name, room_id, room.current_players, room.max_players);
                    }
                }
            },
            
            RoomMessage::ListRooms => {
                let rooms = room_registry.get_room_list();
                let _ = server.send_message::<RoomMessage, Channel1>(
                    *client_id,
                    RoomMessage::RoomList { rooms }
                );
            },
            
            _ => {} // Handle other messages as needed
        }
    }
}

// Handle matchmaking requests  
fn handle_matchmaking(
    mut matchmaking_queue: ResMut<MatchmakingQueue>,
    mut room_registry: ResMut<RoomRegistry>,
    mut server: ResMut<Server>,
    mut events: EventReader<MessageEvent<RoomMessage>>,
    time: Res<Time>,
) {
    for event in events.read() {
        let client_id = event.context();
        let message = &event.message;
        
        if let RoomMessage::StartMatchmaking { game_mode } = message {
            info!("Player {} starting matchmaking for mode: {}", client_id, game_mode);
            
            // Add player to queue
            matchmaking_queue.add_player(
                game_mode.clone(), 
                client_id.to_string(), 
                time.elapsed_secs_f64()
            );
            
            // Try to create a match
            if let Some(matched_players) = matchmaking_queue.try_create_match(game_mode) {
                // Create a room for matched players
                let room_id = format!("MATCH-{}", uuid::Uuid::new_v4().simple());
                let host_name = "Matchmaker".to_string();
                
                let room_data = room_registry.create_room(
                    room_id.clone(), 
                    host_name.clone(), 
                    game_mode.clone()
                );
                
                let room_info = RoomInfo {
                    room_id: room_data.room_id,
                    current_players: matched_players.len() as u32,
                    max_players: room_data.max_players,
                    host_name: room_data.host_name,
                    game_mode: room_data.game_mode,
                };
                
                // Notify all matched players
                for player in matched_players {
                    if let Ok(client_id) = player.player_id.parse::<ClientId>() {
                        let _ = server.send_message::<RoomMessage, Channel1>(
                            client_id,
                            RoomMessage::MatchFound { room_info: room_info.clone() }
                        );
                    }
                }
                
                info!("Created match room {} for {} players in mode {}", 
                      room_id, room_info.current_players, game_mode);
            }
        }
    }
}
