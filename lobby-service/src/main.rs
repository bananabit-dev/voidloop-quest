use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::Filter;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerLobbyRoom {
    id: String,
    host_name: String,
    game_mode: String,
    created_at: u64,
    started: bool,
    current_players: u32,
    max_players: u32,
}

#[derive(Debug, Deserialize)]
struct CreateRoomRequest {
    host_name: String,
    game_mode: String,
    max_players: u32,
}

#[derive(Debug, Deserialize)]
struct LeaveRoomRequest {
    player_name: String,
}

type RoomStorage = Arc<RwLock<HashMap<String, ServerLobbyRoom>>>;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let rooms: RoomStorage = Arc::new(RwLock::new(HashMap::new()));

    // CORS configuration
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "OPTIONS"]);

    // Health check endpoint
    let health = warp::path("health")
        .and(warp::get())
        .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));

    // GET /api/rooms - List all rooms
    let rooms_list = warp::path!("api" / "rooms")
        .and(warp::get())
        .and(with_rooms(rooms.clone()))
        .and_then(list_rooms);

    // POST /api/rooms - Create a new room
    let rooms_create = warp::path!("api" / "rooms")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_rooms(rooms.clone()))
        .and_then(create_room);

    // POST /api/rooms/{id}/leave - Leave a room
    let rooms_leave = warp::path!("api" / "rooms" / String / "leave")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_rooms(rooms.clone()))
        .and_then(leave_room);

    let routes = health
        .or(rooms_list)
        .or(rooms_create)
        .or(rooms_leave)
        .with(cors)
        .with(warp::log("lobby_service"));

    tracing::info!("🏠 Lobby service starting on port 3001");
    warp::serve(routes)
        .run(([0, 0, 0, 0], 3001))
        .await;
}

fn with_rooms(rooms: RoomStorage) -> impl Filter<Extract = (RoomStorage,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || rooms.clone())
}

async fn list_rooms(rooms: RoomStorage) -> Result<impl warp::Reply, warp::Rejection> {
    let rooms = rooms.read();
    let room_list: Vec<ServerLobbyRoom> = rooms.values().cloned().collect();
    tracing::info!("📋 Listing {} rooms", room_list.len());
    Ok(warp::reply::json(&room_list))
}

async fn create_room(
    req: CreateRoomRequest,
    rooms: RoomStorage,
) -> Result<impl warp::Reply, warp::Rejection> {
    let room_id = Uuid::new_v4().to_string();
    let room = ServerLobbyRoom {
        id: room_id.clone(),
        host_name: req.host_name.clone(),
        game_mode: req.game_mode,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        started: false,
        current_players: 1, // Host counts as first player
        max_players: req.max_players,
    };

    rooms.write().insert(room_id.clone(), room.clone());
    tracing::info!("🚀 Created room {} hosted by {}", room_id, req.host_name);
    
    Ok(warp::reply::json(&room))
}

async fn leave_room(
    room_id: String,
    req: LeaveRoomRequest,
    rooms: RoomStorage,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut rooms_guard = rooms.write();
    
    if let Some(room) = rooms_guard.get_mut(&room_id) {
        if room.current_players > 0 {
            room.current_players -= 1;
            tracing::info!("👋 Player {} left room {}, {} players remaining", 
                         req.player_name, room_id, room.current_players);
        }
        
        // Remove empty rooms
        if room.current_players == 0 {
            rooms_guard.remove(&room_id);
            tracing::info!("🗑️ Removed empty room {}", room_id);
        }
        
        Ok(warp::reply::with_status("OK", warp::http::StatusCode::OK))
    } else {
        tracing::warn!("❌ Room {} not found for leave request", room_id);
        Ok(warp::reply::with_status("Room not found", warp::http::StatusCode::NOT_FOUND))
    }
}