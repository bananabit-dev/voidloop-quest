use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;
use warp::Filter;

#[derive(Parser)]
#[command(name = "lobby-server")]
#[command(about = "Simple lobby HTTP server for Voidloop Quest")]
struct Args {
    #[arg(short, long, default_value = "3001")]
    port: u16,
    
    #[arg(short, long, default_value = "0.0.0.0")]
    host: String,
}

// Server-side lobby room representation (matches what client expects)
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ServerLobbyRoom {
    id: String,
    host_name: String,
    game_mode: String,
    created_at: u64,
    started: bool,
    current_players: u32,
    max_players: u32,
}

#[derive(Deserialize)]
struct CreateRoomRequest {
    host_name: String,
    game_mode: String,
    max_players: u32,
}

#[derive(Deserialize)]
struct LeaveRoomRequest {
    player_name: String,
}

// In-memory room storage
type Rooms = Arc<RwLock<HashMap<String, ServerLobbyRoom>>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    let args = Args::parse();
    
    // Initialize empty room storage
    let rooms: Rooms = Arc::new(RwLock::new(HashMap::new()));
    
    info!("🏠 Starting lobby server on {}:{}", args.host, args.port);
    
    // CORS headers for web clients
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "OPTIONS"]);
    
    // GET /lobby/api/rooms - List all rooms
    let rooms_list = warp::path!("lobby" / "api" / "rooms")
        .and(warp::get())
        .and(with_rooms(rooms.clone()))
        .and_then(handle_list_rooms);
    
    // POST /lobby/api/rooms - Create a new room
    let rooms_create = warp::path!("lobby" / "api" / "rooms")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_rooms(rooms.clone()))
        .and_then(handle_create_room);
    
    // POST /lobby/api/rooms/{room_id}/leave - Leave a room
    let rooms_leave = warp::path!("lobby" / "api" / "rooms" / String / "leave")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_rooms(rooms.clone()))
        .and_then(handle_leave_room);
    
    // Health check endpoint
    let health = warp::path!("lobby" / "health")
        .and(warp::get())
        .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));
    
    let routes = rooms_list
        .or(rooms_create)
        .or(rooms_leave)
        .or(health)
        .with(cors)
        .with(warp::log("lobby-server"));
    
    let addr = format!("{}:{}", args.host, args.port);
    info!("🚀 Lobby server running on http://{}", addr);
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], args.port))
        .await;
}

fn with_rooms(rooms: Rooms) -> impl Filter<Extract = (Rooms,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || rooms.clone())
}

async fn handle_list_rooms(rooms: Rooms) -> Result<impl warp::Reply, warp::Rejection> {
    let rooms_guard = rooms.read().await;
    let room_list: Vec<ServerLobbyRoom> = rooms_guard.values().cloned().collect();
    
    info!("📋 Listing {} rooms", room_list.len());
    Ok(warp::reply::json(&room_list))
}

async fn handle_create_room(
    req: CreateRoomRequest,
    rooms: Rooms,
) -> Result<impl warp::Reply, warp::Rejection> {
    let room_id = Uuid::new_v4().to_string();
    let room = ServerLobbyRoom {
        id: room_id.clone(),
        host_name: req.host_name.clone(),
        game_mode: req.game_mode.clone(),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        started: false,
        current_players: 1, // Host is the first player
        max_players: req.max_players,
    };
    
    let mut rooms_guard = rooms.write().await;
    rooms_guard.insert(room_id.clone(), room.clone());
    
    info!("🏠 Created room '{}' hosted by '{}' for game mode '{}'", 
          room_id, req.host_name, req.game_mode);
    
    Ok(warp::reply::json(&room))
}

async fn handle_leave_room(
    room_id: String,
    req: LeaveRoomRequest,
    rooms: Rooms,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut rooms_guard = rooms.write().await;
    
    if let Some(room) = rooms_guard.get_mut(&room_id) {
        if room.current_players > 0 {
            room.current_players -= 1;
        }
        
        info!("👋 Player '{}' left room '{}'", req.player_name, room_id);
        
        // Remove room if empty
        if room.current_players == 0 {
            rooms_guard.remove(&room_id);
            info!("🗑️ Removed empty room '{}'", room_id);
        }
        
        Ok(warp::reply::with_status("OK", warp::http::StatusCode::OK))
    } else {
        warn!("❌ Room '{}' not found for leave request", room_id);
        Ok(warp::reply::with_status(
            "Room not found",
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}