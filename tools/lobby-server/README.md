# Lobby Server

A simple HTTP server that provides lobby functionality for Voidloop Quest. This server handles room creation, listing, and management for the game client.

## API Endpoints

The lobby server provides the following REST API endpoints:

### Health Check
- `GET /lobby/health` - Returns "OK" if the server is running

### Room Management
- `GET /lobby/api/rooms` - List all available rooms
- `POST /lobby/api/rooms` - Create a new room
- `POST /lobby/api/rooms/{room_id}/leave` - Leave a room

## Room Data Structure

Rooms are represented as JSON objects with the following structure:

```json
{
  "id": "uuid-string",
  "host_name": "string",
  "game_mode": "string", 
  "created_at": 1234567890,
  "started": false,
  "current_players": 1,
  "max_players": 4
}
```

## API Usage Examples

### Create a Room
```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"host_name":"MyHost","game_mode":"casual","max_players":4}' \
  http://localhost:3001/lobby/api/rooms
```

### List Rooms
```bash
curl http://localhost:3001/lobby/api/rooms
```

### Leave a Room
```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"player_name":"MyPlayer"}' \
  http://localhost:3001/lobby/api/rooms/{room_id}/leave
```

## Running the Server

### Local Development
```bash
# Build and run directly
cargo run -p lobby-server

# Or with custom host/port
cargo run -p lobby-server -- --host 0.0.0.0 --port 3001
```

### Docker
```bash
# Build the Docker image
docker build -f tools/lobby-server/Dockerfile -t voidloop-quest-lobby .

# Run the container
docker run -p 3001:3001 voidloop-quest-lobby
```

### Docker Compose
The lobby server is included in both development and production Docker Compose configurations:

```bash
# Local development
docker-compose -f docker-compose.localdev.yml up lobby

# Production
docker-compose -f docker-compose.prod.yml up lobby
```

## Configuration

The server accepts the following command-line arguments:

- `--host <HOST>` - Host to bind to (default: 0.0.0.0)
- `--port <PORT>` - Port to listen on (default: 3001)

Environment variables:
- `RUST_LOG` - Set logging level (e.g., "info", "debug")

## Integration with Game Client

The game client automatically detects and uses the lobby server when the `bevygap` feature is enabled. The client makes HTTP requests to:

1. `{host}/lobby/api/rooms` - to fetch available rooms
2. `{host}/lobby/api/rooms` - to create new rooms  
3. `{host}/lobby/api/rooms/{id}/leave` - to leave rooms

## CORS Support

The server includes CORS headers to allow web clients to access the API from different origins.

## Room Lifecycle

1. **Creation** - Rooms are created with 1 player (the host)
2. **Listing** - Active rooms appear in the room list
3. **Leaving** - Players can leave rooms, reducing the player count
4. **Cleanup** - Empty rooms are automatically removed

## Logging

The server logs all API requests and room operations. Set `RUST_LOG=info` for standard logging or `RUST_LOG=debug` for detailed information.