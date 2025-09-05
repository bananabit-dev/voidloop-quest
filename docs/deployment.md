# 🚀 Voidloop Quest Deployment Guide with BevyGap Integration

## 📋 Table of Contents

- [🎯 Overview](#-overview)
- [🔧 Requirements](#-requirements)
- [🏗️ Architecture](#️-architecture)
- [🐳 Docker Deployment](#-docker-deployment)
- [🌐 Domain Setup (voidloop.quest)](#-domain-setup-voidloopquest)
- [🎮 BevyGap Matchmaking & Lobby](#-bevygap-matchmaking--lobby)
- [⚙️ Configuration](#️-configuration)
- [🚦 Quick Start](#-quick-start)
- [📊 Production Deployment](#-production-deployment)
- [🐛 Troubleshooting](#-troubleshooting)
- [📚 Additional Resources](#-additional-resources)

## 🎯 Overview

This guide consolidates all deployment information for **Voidloop Quest** - a multiplayer game using:
- **🎮 Bevy Engine** - Game framework
- **🌐 Lightyear** - Networking library with WebTransport support
- **🔌 BevyGap** - Multiplayer server management & matchmaking
- **☁️ Edgegap** - Global game server hosting
- **🏠 voidloop.quest** - Your game domain with lobby support

## 🔧 Requirements

### 📦 Core Dependencies

```bash
# System Requirements
✅ Rust toolchain (stable/nightly)
✅ Docker 20.10+ & Docker Compose 2.0+
✅ Node.js 18+ (for WASM builds)
✅ wasm-bindgen-cli
✅ Git

# Service Requirements
✅ NATS Server 2.10+ (messaging backbone)
✅ PostgreSQL 15+ (optional, for persistent lobbies)
✅ Redis 7+ (optional, for session caching)
✅ Edgegap Account & API Key
```

### 🔑 API Keys & Secrets

```bash
# Required Environment Variables
EDGEGAP_API_KEY="token-xxxxx-xxxxx-xxxxx"  # From Edgegap dashboard (for server orchestration)
LOBBY_API_KEY=$(openssl rand -hex 32)  # Generated key for internal server-to-lobby auth
LIGHTYEAR_PRIVATE_KEY="1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32"  # 32 bytes
LIGHTYEAR_PROTOCOL_ID="80085"  # Your game's protocol ID
JWT_SECRET=$(openssl rand -hex 32)  # For secure tokens
```

## 🏗️ Architecture

```mermaid
graph TB
    subgraph "🌐 voidloop.quest Domain"
        Client[🎮 Game Client<br/>WASM/Native]
        Web[🌐 Web Server<br/>nginx:80/443]
    end
    
    subgraph "🎯 BevyGap Services"
        HTTPD[📡 Matchmaker HTTPD<br/>WS:3000]
        MM[🎲 Matchmaker<br/>NATS Service]
        NATS[📬 NATS Server<br/>4222]
        Lobby[🏠 Lobby Service<br/>HTTP:3001]
    end
    
    subgraph "☁️ Edgegap Cloud"
        GS1[🖥️ Game Server 1]
        GS2[🖥️ Game Server 2]
        GSN[🖥️ Game Server N]
    end
    
    Client -->|HTTPS| Web
    Client -->|WebSocket| HTTPD
    HTTPD --> MM
    MM --> NATS
    MM -->|API| Edgegap[Edgegap API]
    Edgegap --> GS1
    Edgegap --> GS2
    Edgegap --> GSN
    Client -->|WebTransport| GS1
    Lobby --> NATS
```

## 🐳 Docker Deployment

### 📁 Project Structure

```
workworktokitoki/
├── 📂 work/
│   ├── 📂 voidloop-quest/
│   │   ├── 📂 client/
│   │   │   ├── Dockerfile
│   │   │   └── src/
│   │   ├── 📂 server/
│   │   │   ├── Dockerfile
│   │   │   └── src/
│   │   └── Cargo.toml
│   └── 📂 bevygap/
│       ├── 📂 bevygap_matchmaker/
│       ├── 📂 bevygap_matchmaker_httpd/
│       ├── 📂 bevygap_server_plugin/
│       ├── 📂 bevygap_client_plugin/
│       └── docker-compose.yaml
└── 📄 deployment.md (this file)
```

### 🔨 Building Images

```bash
# 1️⃣ Build Voidloop Quest Client (WASM)
cd work/voidloop-quest
DOCKER_BUILDKIT=1 docker build \
  -f client/Dockerfile \
  -t voidloop-quest-client:latest .

# 2️⃣ Build Voidloop Quest Server
DOCKER_BUILDKIT=1 docker build \
  -f server/Dockerfile \
  -t voidloop-quest-server:latest .

# 3️⃣ Build BevyGap Services
cd ../bevygap
docker compose build
```

### 🐋 Docker Compose Configuration

Create `docker-compose.yaml` in your project root:

```yaml
version: '3.8'

services:
  # 📬 NATS - Message Broker
  nats:
    image: nats:2.10.21
    ports:
      - "4222:4222"
    restart: unless-stopped
    volumes:
      - ./nats-data:/data
    healthcheck:
      test: ["CMD", "nc", "-z", "localhost", "4222"]
      interval: 5s
      timeout: 3s
      retries: 5

  # 🌐 Game Client Web Server
  voidloop-client:
    image: voidloop-quest-client:latest
    ports:
      - "80:80"
      - "443:443"
    environment:
      - MATCHMAKER_URL=wss://voidloop.quest/matchmaker/ws
    volumes:
      - ./ssl:/etc/nginx/ssl:ro
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.client.rule=Host(`voidloop.quest`)"
      - "traefik.http.services.client.loadbalancer.server.port=80"
    restart: unless-stopped

  # 📡 BevyGap Matchmaker HTTPD
  matchmaker-httpd:
    image: bevygap_matchmaker_httpd
    build:
      context: ./work/bevygap
      dockerfile: ./bevygap_matchmaker_httpd/Dockerfile
    ports:
      - "3000:3000"
    environment:
      NATS_USER: "matchmaker_httpd"
      NATS_PASS: "matchmaker_httpd"
      NATS_HOST: "nats"
      NATS_INSECURE: "set"
    command: >
      --bind 0.0.0.0:3000
      --cors https://voidloop.quest
      --player-limit 16
      --fake-ip 127.0.0.1
    depends_on:
      nats:
        condition: service_healthy
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.matchmaker.rule=Host(`voidloop.quest`) && PathPrefix(`/matchmaker`)"
      - "traefik.http.services.matchmaker.loadbalancer.server.port=3000"
    restart: unless-stopped

  # 🎲 BevyGap Matchmaker Service
  matchmaker:
    image: bevygap_matchmaker
    build:
      context: ./work/bevygap
      dockerfile: ./bevygap_matchmaker/Dockerfile
    environment:
      NATS_USER: "matchmaker"
      NATS_PASS: "matchmaker"
      NATS_HOST: "nats"
      NATS_INSECURE: "set"
      EDGEGAP_API_KEY: "${EDGEGAP_API_KEY}"
    command: >
      --app-name voidloop-quest
      --app-version 1
      --lightyear-protocol-id ${LIGHTYEAR_PROTOCOL_ID}
      --lightyear-private-key "${LIGHTYEAR_PRIVATE_KEY}"
      --player-limit 16
    depends_on:
      nats:
        condition: service_healthy
    restart: unless-stopped

  # 🏠 Lobby Service (Optional)
  lobby:
    image: bevygap_lobby
    build:
      context: ./work/bevygap
      dockerfile: ./bevygap_lobby/Dockerfile
    ports:
      - "3001:3001"
    environment:
      DATABASE_URL: "postgres://lobby:${DB_PASSWORD}@postgres:5432/voidloop_lobby"
      JWT_SECRET: "${JWT_SECRET}"
      NATS_HOST: "nats"
    depends_on:
      - nats
      - postgres
    restart: unless-stopped

  # 🗄️ PostgreSQL for Lobby
  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_USER: lobby
      POSTGRES_PASSWORD: "${DB_PASSWORD}"
      POSTGRES_DB: voidloop_lobby
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

  # 🔄 Traefik Reverse Proxy (Optional, for SSL)
  traefik:
    image: traefik:3.2
    ports:
      - "443:443"
      - "8080:8080"  # Dashboard
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - ./traefik.yml:/traefik.yml
      - ./ssl:/ssl:ro
    restart: unless-stopped

volumes:
  nats-data:
  postgres_data:
```

## 🌐 Domain Setup (voidloop.quest)

### 🔒 SSL Certificate Setup

```bash
# Option 1: Let's Encrypt with Certbot
sudo certbot certonly --standalone -d voidloop.quest -d www.voidloop.quest

# Option 2: Using Traefik (automatic)
# Add to traefik.yml:
certificatesResolvers:
  letsencrypt:
    acme:
      email: admin@voidloop.quest
      storage: /ssl/acme.json
      httpChallenge:
        entryPoint: web
```

### 🌍 DNS Configuration

Configure your DNS records at your domain provider:

```
Type    Name    Value                   TTL
A       @       YOUR_SERVER_IP          300
A       www     YOUR_SERVER_IP          300
A       game    YOUR_GAME_SERVER_IP     300
CNAME   lobby   voidloop.quest          300
```

### 🔧 Nginx Configuration for voidloop.quest

```nginx
# /etc/nginx/sites-available/voidloop.quest
server {
    listen 80;
    server_name voidloop.quest www.voidloop.quest;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name voidloop.quest www.voidloop.quest;

    # 🔒 SSL Configuration
    ssl_certificate /etc/letsencrypt/live/voidloop.quest/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/voidloop.quest/privkey.pem;
    
    # 🔐 Security Headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Cross-Origin-Embedder-Policy "require-corp" always;
    add_header Cross-Origin-Opener-Policy "same-origin" always;

    # 🎮 Game Client
    location / {
        proxy_pass http://localhost:80;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # 📡 WebSocket for Matchmaker
    location /matchmaker/ws {
        proxy_pass http://localhost:3000/matchmaker/ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
    }

    # 🏠 Lobby API (served by webhook_sink service)
    location /lobby/ {
        proxy_pass http://localhost:3001/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## 🎮 BevyGap Matchmaking & Lobby

### 🔌 Required BevyGap Components

```yaml
# ✅ Core Services Required
bevygap_matchmaker         # Handles Edgegap session creation
bevygap_matchmaker_httpd   # WebSocket endpoint for clients
bevygap_server_plugin      # Server-side integration
bevygap_client_plugin      # Client-side integration
bevy_nfws                  # WebSocket client library

# 📦 Dependencies
NATS Server 2.10+          # Message broker
Edgegap Account            # Game server hosting
```

### 🎯 Lobby Configuration

```rust
// 🎮 Client-side lobby configuration
// client/src/screens/lobby.rs
use bevygap_client_plugin::{BevyGapClientPlugin, ClientConfig};

pub struct LobbyConfig {
    pub domain: String,           // "voidloop.quest"
    pub matchmaker_url: String,   // "wss://voidloop.quest/matchmaker/ws"
    pub max_players: u32,         // 16
    pub lobby_modes: Vec<String>, // ["casual", "ranked", "custom"]
}

impl Default for LobbyConfig {
    fn default() -> Self {
        Self {
            domain: "voidloop.quest".to_string(),
            matchmaker_url: "wss://voidloop.quest/matchmaker/ws".to_string(),
            max_players: 16,
            lobby_modes: vec![
                "casual".to_string(),
                "ranked".to_string(),
                "custom".to_string(),
            ],
        }
    }
}

// 🏠 Initialize lobby system
fn setup_lobby(mut commands: Commands) {
    commands.insert_resource(LobbyConfig::default());
    commands.spawn(LobbyUI::new());
}
```

### 🔄 Matchmaking Flow

```mermaid
sequenceDiagram
    participant C as 🎮 Client
    participant L as 🏠 Lobby UI
    participant M as 📡 Matchmaker HTTPD
    participant MM as 🎲 Matchmaker
    participant E as ☁️ Edgegap
    participant G as 🖥️ Game Server

    C->>L: Select game mode & players
    L->>M: Connect WebSocket
    M->>MM: Request session
    MM->>E: Create/find session
    E->>G: Deploy server
    G-->>MM: Server ready
    MM-->>M: Session details
    M-->>L: Connection info
    L-->>C: Connect token
    C->>G: WebTransport connect
    G-->>C: Game starts! 🎮
```

## ⚙️ Configuration

### 🔐 Environment Variables - Complete Setup Guide

#### 📍 **WHERE to Set Environment Variables:**

1. **Local Development** → `.env` file in project root
2. **Docker Compose** → `.env` file + `docker-compose.yaml` environment sections
3. **Kubernetes** → ConfigMaps and Secrets
4. **Edgegap Dashboard** → Application Version > Environment Variables
5. **System** → `/etc/environment` or `~/.bashrc`

#### 📝 **WHAT Variables to Set & WHERE:**

Create `.env.production` file:

```bash
# ============================================
# 🔑 EDGEGAP CONFIGURATION (Required)
# ============================================
# WHERE: Set in multiple places
# 1. Your local .env file (for matchmaker)
# 2. Docker compose environment
# 3. Edgegap Dashboard > App Version > Environment Variables
EDGEGAP_API_KEY="token-xxxxx-xxxxx-xxxxx"  # Get from https://app.edgegap.com/
EDGEGAP_APP_NAME="voidloop-quest"
EDGEGAP_APP_VERSION="1"

# ============================================
# 🔒 MUTUAL TLS (mTLS) CONFIGURATION
# ============================================
# WHERE: All services that communicate
# WHAT: Paths to certificates for secure server-to-lobby communication
LOBBY_USE_MTLS=true
LOBBY_CA_CERT=/certs/ca.crt              # Certificate Authority
LOBBY_SERVER_CERT=/certs/lobby.crt       # Lobby's certificate
LOBBY_SERVER_KEY=/certs/lobby.key        # Lobby's private key
GAME_SERVER_CERT=/certs/game-server.crt  # Game server's certificate
GAME_SERVER_KEY=/certs/game-server.key   # Game server's private key

# ============================================
# 🔐 LIGHTYEAR SECURITY
# ============================================
# WHERE: 
# 1. Matchmaker service (generates tokens)
# 2. Game servers (validates connections)
# 3. Edgegap Dashboard > Environment Variables
LIGHTYEAR_PROTOCOL_ID="80085"
LIGHTYEAR_PRIVATE_KEY="1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32"

# ============================================
# 🌐 SERVICE URLS
# ============================================
# WHERE: Client builds, game servers, all services
DOMAIN="voidloop.quest"
MATCHMAKER_URL="wss://voidloop.quest/matchmaker/ws"
LOBBY_URL="https://voidloop.quest/lobby"  # Served by webhook_sink service
NATS_URL="nats://nats:4222"

# ============================================
# 🗄️ DATABASE
# ============================================
# WHERE: Lobby service, backup scripts
DB_PASSWORD=$(openssl rand -hex 32)
DATABASE_URL="postgres://lobby:${DB_PASSWORD}@postgres:5432/voidloop_lobby"

# ============================================
# 📬 NATS CONFIGURATION
# ============================================
# WHERE: All BevyGap services
NATS_USER="matchmaker"
NATS_PASSWORD=$(openssl rand -hex 32)
NATS_HOST="nats"
NATS_PORT="4222"
NATS_USE_TLS=true
NATS_CA_CERT=/certs/nats-ca.crt

# ============================================
# 📊 GAME SETTINGS
# ============================================
# WHERE: Game servers (via Edgegap env vars)
MAX_PLAYERS=16
TICK_RATE=60
SESSION_TIMEOUT=300
SERVER_REGION="auto"  # or specific like "us-east"

# ============================================
# 🔐 ADDITIONAL SECURITY
# ============================================
JWT_SECRET=$(openssl rand -hex 64)
CORS_ORIGIN="https://voidloop.quest"
RATE_LIMIT_PER_IP=100  # requests per minute
```

### 🔒 **mTLS Certificate Generation Script**

Create `scripts/generate-mtls-certs.sh`:

```bash
#!/bin/bash
# Generate mTLS certificates for production deployment

CERT_DIR="./certs"
mkdir -p $CERT_DIR

# 1️⃣ Generate Certificate Authority (CA)
echo "🔐 Generating Certificate Authority..."
openssl req -x509 -newkey rsa:4096 -days 365 -nodes \
  -keyout $CERT_DIR/ca.key \
  -out $CERT_DIR/ca.crt \
  -subj "/C=US/ST=State/L=City/O=VoidloopQuest/CN=VoidloopCA"

# 2️⃣ Generate Lobby Server Certificate
echo "🏠 Generating Lobby Server Certificate..."
openssl req -newkey rsa:4096 -nodes \
  -keyout $CERT_DIR/lobby.key \
  -out $CERT_DIR/lobby.csr \
  -subj "/C=US/ST=State/L=City/O=VoidloopQuest/CN=lobby.voidloop.quest"

openssl x509 -req -days 365 \
  -in $CERT_DIR/lobby.csr \
  -CA $CERT_DIR/ca.crt \
  -CAkey $CERT_DIR/ca.key \
  -CAcreateserial \
  -out $CERT_DIR/lobby.crt

# 3️⃣ Generate Game Server Certificate Template
echo "🎮 Generating Game Server Certificate Template..."
openssl req -newkey rsa:4096 -nodes \
  -keyout $CERT_DIR/game-server.key \
  -out $CERT_DIR/game-server.csr \
  -subj "/C=US/ST=State/L=City/O=VoidloopQuest/CN=game-server"

openssl x509 -req -days 365 \
  -in $CERT_DIR/game-server.csr \
  -CA $CERT_DIR/ca.crt \
  -CAkey $CERT_DIR/ca.key \
  -CAcreateserial \
  -out $CERT_DIR/game-server.crt

# 4️⃣ Generate NATS Certificate
echo "📬 Generating NATS Certificate..."
openssl req -newkey rsa:4096 -nodes \
  -keyout $CERT_DIR/nats.key \
  -out $CERT_DIR/nats.csr \
  -subj "/C=US/ST=State/L=City/O=VoidloopQuest/CN=nats.voidloop.quest"

openssl x509 -req -days 365 \
  -in $CERT_DIR/nats.csr \
  -CA $CERT_DIR/ca.crt \
  -CAkey $CERT_DIR/ca.key \
  -CAcreateserial \
  -out $CERT_DIR/nats.crt

# 5️⃣ Set Permissions
chmod 600 $CERT_DIR/*.key
chmod 644 $CERT_DIR/*.crt

echo "✅ Certificates generated in $CERT_DIR/"
echo "📋 Files created:"
ls -la $CERT_DIR/
```

### 🎮 Game Server Configuration

```toml
# server/config.toml
[server]
host = "0.0.0.0"
port = 6420
transport_port = 6421
max_players = 16
tick_rate = 60

[bevygap]
enable = true
nats_url = "nats://nats:4222"
report_cert_digest = true
heartbeat_interval = 30

[lightyear]
protocol_id = 80085
compression = "lz4"
```

## 🚦 Quick Start

### 🏃 Local Development

```bash
# 1️⃣ Clone and setup
git clone https://github.com/your-org/voidloop-quest.git
cd voidloop-quest

# 2️⃣ Install dependencies
cargo install wasm-bindgen-cli
cargo install cargo-watch
npm install -g http-server

# 3️⃣ Start NATS (required)
docker run -d --name nats -p 4222:4222 nats:2.10.21

# 4️⃣ Start BevyGap services
cd work/bevygap
cargo run -p bevygap_matchmaker_httpd &
cargo run -p bevygap_matchmaker -- \
  --app-name voidloop-quest \
  --app-version 1 \
  --lightyear-protocol-id 80085 \
  --lightyear-private-key "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32" &

# 5️⃣ Build and run client
cd ../voidloop-quest
cargo build --package client --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/debug/client.wasm \
  --out-dir www --target web
cd www && http-server -p 8080

# 6️⃣ Access game at http://localhost:8080
```

### 🚀 Production Deployment

```bash
# 1️⃣ Set production environment
export ENVIRONMENT=production
source .env.production

# 2️⃣ Build all images
./scripts/build-all.sh

# 3️⃣ Deploy with Docker Compose
docker compose -f docker-compose.prod.yaml up -d

# 4️⃣ Setup SSL certificates
sudo certbot certonly --webroot \
  -w /var/www/voidloop.quest \
  -d voidloop.quest \
  -d www.voidloop.quest

# 5️⃣ Configure Edgegap
# - Login to Edgegap dashboard
# - Create app: "voidloop-quest"
# - Upload server image
# - Configure ports: 6420/udp, 6421/tcp
# - Set environment variables

# 6️⃣ Verify deployment
curl https://voidloop.quest/health
curl https://voidloop.quest/matchmaker/health
curl https://voidloop.quest/lobby/health
```

## 📊 Production Deployment

### ☸️ Kubernetes Deployment

```yaml
# k8s/voidloop-quest-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: voidloop-quest
  namespace: games
spec:
  replicas: 3
  selector:
    matchLabels:
      app: voidloop-quest
  template:
    metadata:
      labels:
        app: voidloop-quest
    spec:
      containers:
      - name: client
        image: voidloop-quest-client:latest
        ports:
        - containerPort: 80
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "256Mi"
            cpu: "200m"
---
apiVersion: v1
kind: Service
metadata:
  name: voidloop-quest
  namespace: games
spec:
  selector:
    app: voidloop-quest
  ports:
  - port: 80
    targetPort: 80
  type: LoadBalancer
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: voidloop-quest
  namespace: games
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/websocket-services: matchmaker-httpd
spec:
  tls:
  - hosts:
    - voidloop.quest
    secretName: voidloop-quest-tls
  rules:
  - host: voidloop.quest
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: voidloop-quest
            port:
              number: 80
      - path: /matchmaker
        pathType: Prefix
        backend:
          service:
            name: matchmaker-httpd
            port:
              number: 3000
```

### 📈 Monitoring & Observability

```yaml
# docker-compose.monitoring.yaml
services:
  # 📊 Prometheus
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    ports:
      - "9090:9090"
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'

  # 📈 Grafana
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3002:3000"
    volumes:
      - grafana_data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_PASSWORD}
      - GF_SERVER_ROOT_URL=https://voidloop.quest/grafana

  # 📝 Loki (Logs)
  loki:
    image: grafana/loki:latest
    ports:
      - "3100:3100"
    volumes:
      - loki_data:/loki

volumes:
  prometheus_data:
  grafana_data:
  loki_data:
```

### 🔐 Security Checklist

```bash
✅ SSL/TLS enabled for all endpoints
✅ CORS properly configured
✅ Rate limiting enabled
✅ DDoS protection (Cloudflare/AWS Shield)
✅ Secrets in environment variables
✅ Container security scanning
✅ Non-root container users
✅ Network isolation
✅ Regular security updates
✅ Backup strategy implemented
```

### 🔒 Enhanced Webhook Sink & Lobby Security Options

The webhook_sink service provides both webhook processing and lobby API endpoints. It can be secured using several approaches:

#### Option 1: Public Lobby API with Rate Limiting
```nginx
# Allow public access to lobby API with rate limiting
location /lobby/api/ {
    limit_req zone=lobby_limit burst=20 nodelay;
    proxy_pass http://webhook_sink:3001;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
}
```

#### Option 2: Internal Service Security
- Service runs on internal Docker network
- NATS authentication with username/password
- Database access secured with strong passwords
- Webhook endpoints can be restricted to internal network only


## 🐛 Troubleshooting

### ❌ Common Issues & Solutions

#### 🔴 WebSocket Connection Failed
```bash
# Check if matchmaker is running
docker logs matchmaker-httpd

# Verify WebSocket upgrade headers
curl -i -N -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: SGVsbG8sIHdvcmxkIQ==" \
  https://voidloop.quest/matchmaker/ws

# Solution: Ensure nginx/traefik properly forwards WebSocket
```

#### 🔴 Edgegap Session Creation Failed
```bash
# Check Edgegap API key
curl -H "Authorization: token $EDGEGAP_API_KEY" \
  https://api.edgegap.com/v1/apps

# Verify app deployment
docker logs matchmaker | grep ERROR

# Solution: Ensure Edgegap app is deployed and API key is valid
```

#### 🔴 WebTransport Certificate Issues
```bash
# Check certificate digest
openssl s_client -connect game.voidloop.quest:6421 </dev/null 2>/dev/null | \
  openssl x509 -fingerprint -sha256 -noout

# Solution: Ensure server publishes correct cert digest
```

#### 🔴 NATS Connection Issues
```bash
# Test NATS connectivity
docker exec -it nats nc -zv localhost 4222

# Check NATS logs
docker logs nats

# Solution: Verify NATS credentials and network connectivity
```

### 🛠️ Debug Commands

```bash
# 📊 Check container status
docker compose ps
docker compose logs -f matchmaker

# 🔍 Inspect network
docker network inspect voidloop_default

# 📈 Monitor resources
docker stats

# 🔐 Check certificates
openssl x509 -in /etc/letsencrypt/live/voidloop.quest/cert.pem -text -noout

# 📡 Test WebSocket
wscat -c wss://voidloop.quest/matchmaker/ws

# 🎮 Test game connection
curl -v https://voidloop.quest/health
```

## 📚 Additional Resources

### 📖 Documentation
- 🎮 [Bevy Engine Documentation](https://bevyengine.org/learn/)
- 🌐 [Lightyear Networking](https://github.com/cBournhonesque/lightyear)
- 🔌 [BevyGap Framework](https://github.com/RJ/bevygap)
- ☁️ [Edgegap Documentation](https://docs.edgegap.com/)

### 🔗 Quick Links
- 🏠 **Game URL**: https://voidloop.quest
- 📡 **Matchmaker**: wss://voidloop.quest/matchmaker/ws
- 🏠 **Lobby API**: https://voidloop.quest/lobby (webhook_sink service)
- 📊 **Monitoring**: https://voidloop.quest/grafana
- 📝 **Logs**: https://voidloop.quest/loki

### 💬 Support
- 📧 Email: support@voidloop.quest
- 💬 Discord: https://discord.gg/voidloop
- 🐛 Issues: https://github.com/your-org/voidloop-quest/issues

---

## 🎉 Summary

You now have a complete deployment setup for **Voidloop.quest** with:

✅ **BevyGap** matchmaking and lobby system  
✅ **Edgegap** global server deployment  
✅ **WebTransport** for low-latency gameplay  
✅ **NATS** for reliable messaging  
✅ **SSL/TLS** security on voidloop.quest domain  
✅ **Docker Compose** for easy deployment  
✅ **Monitoring** with Prometheus/Grafana  
✅ **Production-ready** configuration  

🚀 **Your game is ready to launch at voidloop.quest!**
