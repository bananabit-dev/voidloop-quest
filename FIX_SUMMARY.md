# Fix Summary: WASM Client 404 Errors

## Problem
The WASM client was getting 404 errors when trying to connect to bevygap services:
- `GET wss://voidloop.quest/matchmaker/ws` → HTTP/1.1 404 Not Found
- `POST https://voidloop.quest/hook/api/rooms` → HTTP/2 404 Not Found

## Root Cause
The WASM build script used `--no-default-features` which disabled the `bevygap` feature, but the lobby UI code still attempted to make HTTP requests to bevygap API endpoints without checking if the feature was enabled.

## Solution
1. **Added feature guards** to lobby HTTP API calls:
   - `LobbyEvent::ConfirmCreateRoom` - Room creation
   - `LobbyEvent::RequestRoomList` - Room listing  
   - `LobbyEvent::LeaveRoom` - Room leaving

2. **Added fallback behavior** for WASM builds without bevygap:
   - Create local rooms instead of API calls
   - Use local room registry for room lists
   - Skip HTTP requests for room operations

3. **Updated build script** to support both modes:
   - **Production**: `./build-wasm.sh` (includes bevygap)
   - **Development**: `DISABLE_BEVYGAP=true ./build-wasm.sh` (no external deps)

## Code Changes

### client/src/screens/lobby.rs
```rust
// Before: Always made HTTP requests
#[cfg(target_arch = "wasm32")]
{
    spawn_local(async move {
        let url = format!("{}/hook/api/rooms", http_base());
        // ... HTTP request code
    });
}

// After: Feature-gated with fallback
#[cfg(all(target_arch = "wasm32", feature = "bevygap"))]
{
    spawn_local(async move {
        let url = format!("{}/hook/api/rooms", http_base());
        // ... HTTP request code
    });
}
#[cfg(all(target_arch = "wasm32", not(feature = "bevygap")))]
{
    // Fallback: create local room
    let room_info = RoomInfo { /* ... */ };
    room_registry.rooms.push(room_info);
    info!("🏠 Created local room: {} (bevygap disabled)", room_id);
}
```

### build-wasm.sh
```bash
# Added environment variable support
if [ "${DISABLE_BEVYGAP:-false}" = "true" ]; then
    FEATURES_FLAG="--no-default-features"
else
    FEATURES_FLAG=""
fi
```

## Results
- ✅ No more 404 errors in development mode
- ✅ Production mode still supports bevygap services
- ✅ Graceful fallback to local functionality
- ✅ Both build configurations compile and work correctly

## Testing
- Development build: `DISABLE_BEVYGAP=true ./build-wasm.sh`
- Production build: `./build-wasm.sh`
- Both builds tested and working without errors