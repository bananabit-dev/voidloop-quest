# 🎮 Voidloop Quest - Real Matchmaking Implementation Guide

## Overview

Voidloop Quest now includes **real matchmaking** powered by Edgegap's lobby system and BevyGap integration. This replaces the previous simulated matchmaking with actual game server deployment.

## 🔄 How Real Matchmaking Works

### 1. **User Flow**
```
1. Player selects game mode (casual/ranked/custom)
2. Player clicks "Start Game" in lobby
3. System creates unique Edgegap lobby 
4. Lobby automatically deploys game server
5. BevyGap connects to deployed server
6. Player enters live multiplayer game
```

### 2. **Technical Flow**
```
StartMatchmaking Event
    ↓
Create Edgegap Lobby (voidloop-{mode}-{timestamp})
    ↓
Deploy Lobby → Spawns Game Server
    ↓
LobbyReadResponse contains server URL & status
    ↓
BevyGap connects to server
    ↓
Transition to InGame state
```

## 🛠️ Implementation Details

### **New Components Added**
- `EdgegapLobbyState` resource - tracks lobby deployment
- `StartMatchmaking` event - triggers real matchmaking
- `LobbyCreated`/`LobbyDeployed`/`LobbyDeploymentFailed` events 
- `handle_matchmaking_events` system - orchestrates API calls

### **Configuration**
```bash
# Required environment variables
export EDGEGAP_BASE_URL="https://api.edgegap.com"
export EDGEGAP_TOKEN="your-api-token"
```

### **Lobby Naming Convention**
```
voidloop-{mode}-{timestamp}
# Example: voidloop-casual-1671234567
```

## 🚀 Usage

### **For Development (No Networking)**
```bash
cargo run --no-default-features -p server
cargo run --no-default-features -p client
```

### **For Production (Real Matchmaking with Secure Token Handling)**
```bash
# Set up environment for MATCHMAKER SERVICE ONLY
export EDGEGAP_BASE_URL="https://api.edgegap.com"
export EDGEGAP_TOKEN="your-edgegap-api-token"

# Start the secure matchmaker service (handles Edgegap API)
cargo run --bin matchmaker --features matchmaker

# In another terminal, start the game server (no token needed)
cargo run -p server

# In another terminal, start the client (no token needed)
cargo run -p client
```

**🔐 Security Improvement**: The EDGEGAP_TOKEN is now only required by the matchmaker service, not the client. This prevents token exposure in client-side code.

### **CLI Tool for Lobby Management**
```bash
# List lobbies
cargo run -p lobby -- list

# Create lobby manually
cargo run -p lobby -- create test-lobby

# Deploy lobby (starts server)
cargo run -p lobby -- deploy test-lobby

# Get lobby status
cargo run -p lobby -- get test-lobby

# Clean up
cargo run -p lobby -- terminate test-lobby
cargo run -p lobby -- delete test-lobby
```

## 🎯 Key Features

### **Automatic Server Deployment**
- Creates unique lobby per game session
- Deploys game server automatically via Edgegap
- Returns server URL and connection details

### **Real Connection Management**
- Uses BevyGap for client-server connection
- Leverages LobbyReadResponse for server announcement
- Handles connection failures gracefully

### **Production Ready**
- Environment-based configuration
- Proper error handling and logging
- Integration with Edgegap's infrastructure

## 🔧 Error Handling

### **Common Issues**
1. **EDGEGAP_TOKEN not set** → Shows error in UI
2. **API rate limits** → Graceful degradation 
3. **Server deployment fails** → User feedback
4. **Connection timeouts** → Retry logic

### **Debugging**
```bash
# Enable debug logging
RUST_LOG=debug cargo run -p client

# Check lobby status
cargo run -p lobby -- get lobby-name

# List all active lobbies
cargo run -p lobby -- list
```

## 🌟 Benefits Over Simulated Matchmaking

1. **Real Servers** - Actual game servers, not local simulation
2. **Auto-scaling** - Edgegap handles server provisioning
3. **Global Deployment** - Servers deployed close to players
4. **Production Ready** - Battle-tested infrastructure
5. **Monitoring** - Full visibility into lobby lifecycle

This implementation transforms Voidloop Quest from a local prototype into a production-ready multiplayer game with real server deployment and matchmaking.