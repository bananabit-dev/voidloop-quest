# Lobby Service

A lightweight HTTP service that provides room management API endpoints for Voidloop Quest.

## Purpose

This service handles the lobby/room management functionality that the WASM client expects when players click "Join Room". It provides REST API endpoints for:

- Listing available rooms
- Creating new rooms
- Managing player leave/join operations

## API Endpoints

### GET /api/rooms
Returns a list of all available rooms.

**Response:**
```json
[
  {
    "id": "uuid",
    "host_name": "PlayerName",
    "game_mode": "classic",
    "created_at": 1234567890,
    "started": false,
    "current_players": 1,
    "max_players": 4
  }
]
```

### POST /api/rooms
Creates a new room.

**Request Body:**
```json
{
  "host_name": "PlayerName",
  "game_mode": "classic",
  "max_players": 4
}
```

**Response:**
Returns the created room object (same format as above).

### POST /api/rooms/{room_id}/leave
Removes a player from a room.

**Request Body:**
```json
{
  "player_name": "PlayerName"
}
```

**Response:**
Returns "OK" on success. Empty rooms are automatically removed.

### GET /health
Health check endpoint that returns "OK".

## Running

### Local Development
```bash
cargo run -p lobby-service
```

### Docker
```bash
docker build -t lobby-service -f lobby-service/Dockerfile .
docker run -p 3001:3001 lobby-service
```

## Configuration

The service runs on port 3001 by default and is configured to work with the Caddy proxy setup that routes `/hook/*` requests to this service.

## Features

- **CORS Support**: Allows cross-origin requests from the web client
- **Automatic Cleanup**: Empty rooms are removed when the last player leaves
- **JSON API**: All responses are in JSON format expected by the client
- **Health Checks**: Includes health check endpoint for monitoring