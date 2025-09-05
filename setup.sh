#!/usr/bin/env bash
# Voidloop Quest Enhanced VPS Setup with NATS TLS (Ubuntu/Debian)
# - Installs Docker & Compose
# - Configures UFW firewall and a 2G swapfile
# - Sets up NATS with TLS for Edgegap gameservers
# - Generates secrets and config
# - Creates Caddy reverse proxy (auto TLS), NATS (TLS auth), Matchmaker, Lobby (with Postgres), Client
# - Starts the stack with Docker Compose
#
# Requirements:
# - Run as root (sudo -i) on Ubuntu 20.04+/Debian 11+
# - DNS A/AAAA of your domain points to this VPS
# - DNS A/AAAA of nats.your-domain points to this VPS (for NATS TLS)
#
# Note:
# - You must provide container image names that are accessible (public or docker login beforehand)
# - If your images are private, run `docker login` before executing this script
#

set -euo pipefail

# ---------- Helpers ----------
abort() { echo "Error: $*" >&2; exit 1; }
require_root() { [ "$(id -u)" -eq 0 ] || abort "Run as root (sudo -i)"; }
command_exists() { command -v "$1" >/dev/null 2>&1; }
gen_hex() { openssl rand -hex "${1:-32}"; }
detect_ssh_port() { sshd -T 2>/dev/null | awk '/^port /{print $2; exit}' || echo "22"; }

require_root

echo "=== Voidloop Quest Enhanced VPS Setup with NATS TLS ==="

# ---------- Collect configuration ----------
read -rp "Domain (e.g., voidloop.quest): " DOMAIN
[ -n "${DOMAIN:-}" ] || abort "Domain is required"

read -rp "Admin email for TLS (Let's Encrypt): " EMAIL
[ -n "${EMAIL:-}" ] || abort "Email is required"

echo
echo "IMPORTANT: You need to set up DNS records:"
echo "  1. A/AAAA record for ${DOMAIN} -> this VPS IP"
echo "  2. A/AAAA record for nats.${DOMAIN} -> this VPS IP"
echo "Have you configured both DNS records? [Y/n]: "
read -rp "" DNS_READY
DNS_READY="${DNS_READY:-Y}"
[[ "$DNS_READY" =~ ^[Yy]$ ]] || abort "Please configure DNS first"

echo
echo "Container images configuration:"
echo "  - Using Docker Hub (bananabit/*) for web services"
echo

read -rp "Client image [bananabit/voidloop-quest-wasm:latest]: " CLIENT_IMAGE
CLIENT_IMAGE="${CLIENT_IMAGE:-bananabit/voidloop-quest-wasm:latest}"

read -rp "Matchmaker HTTPD image [bananabit/bevygap_matchmaker_httpd:latest]: " MM_HTTPD_IMAGE
MM_HTTPD_IMAGE="${MM_HTTPD_IMAGE:-bananabit/bevygap_matchmaker_httpd:latest}"

read -rp "Matchmaker image [bananabit/bevygap_matchmaker:latest]: " MM_IMAGE
MM_IMAGE="${MM_IMAGE:-bananabit/bevygap_matchmaker:latest}"

read -rp "Lobby image [bananabit/bevygap_lobby:latest]: " LOBBY_IMAGE
LOBBY_IMAGE="${LOBBY_IMAGE:-bananabit/bevygap_lobby:latest}"

read -rp "Max players per session [4]: " MAX_PLAYERS
MAX_PLAYERS="${MAX_PLAYERS:-4}"

read -rp "Lightyear protocol ID [80085]: " LIGHTYEAR_PROTOCOL_ID
LIGHTYEAR_PROTOCOL_ID="${LIGHTYEAR_PROTOCOL_ID:-80085}"

read -rp "Lightyear private key (32 comma-separated bytes) [paste or leave blank to set later]: " LIGHTYEAR_PRIVATE_KEY
if [ -z "${LIGHTYEAR_PRIVATE_KEY:-}" ]; then
  echo "Warning: LIGHTYEAR_PRIVATE_KEY not set. You must edit .env later before matchmaking works."
fi

read -rsp "Edgegap API Key (paste, input hidden) [leave blank to set later]: " EDGEGAP_API_KEY
echo
if [ -z "${EDGEGAP_API_KEY:-}" ]; then
  echo "Warning: EDGEGAP_API_KEY not set. You must edit .env later before Edgegap orchestration works."
fi

# Generate strong passwords for NATS users
echo "Generating secure NATS passwords..."
NATS_MATCHMAKER_PASSWORD="$(gen_hex 32)"
NATS_MATCHMAKER_HTTPD_PASSWORD="$(gen_hex 32)"
NATS_GAMESERVER_PASSWORD="$(gen_hex 32)"
NATS_LOBBY_PASSWORD="$(gen_hex 32)"

DB_PASSWORD="$(gen_hex 32)"
JWT_SECRET="$(gen_hex 64)"

read -rp "Configure UFW firewall (allow SSH/80/443/4222) and enable? [Y/n]: " CFG_UFW
CFG_UFW="${CFG_UFW:-Y}"

read -rp "Create 2G swapfile if none exists? [Y/n]: " CFG_SWAP
CFG_SWAP="${CFG_SWAP:-Y}"

read -rp "Install fail2ban (SSH protection)? [y/N]: " CFG_F2B
CFG_F2B="${CFG_F2B:-N}"

read -rp "Install unattended-upgrades (auto security updates)? [y/N]: " CFG_UU
CFG_UU="${CFG_UU:-N}"

SSH_PORT="$(detect_ssh_port)"

echo
echo "Summary:"
echo "  Domain:                 $DOMAIN"
echo "  NATS Domain:            nats.$DOMAIN"
echo "  Email:                  $EMAIL"
echo "  Client image:           $CLIENT_IMAGE"
echo "  MM HTTPD image:         $MM_HTTPD_IMAGE"
echo "  MM image:               $MM_IMAGE"
echo "  Lobby image:            $LOBBY_IMAGE"
echo "  Max players:            $MAX_PLAYERS"
echo "  Lightyear Protocol ID:  $LIGHTYEAR_PROTOCOL_ID"
echo "  Lightyear Private Key:  ${LIGHTYEAR_PRIVATE_KEY:+[SET]}${LIGHTYEAR_PRIVATE_KEY:+" (hidden)"}"
echo "  Edgegap API Key:        ${EDGEGAP_API_KEY:+[SET]}${EDGEGAP_API_KEY:+" (hidden)"}"
echo "  SSH port:               $SSH_PORT"
echo "  UFW:                    $CFG_UFW"
echo "  Swap:                   $CFG_SWAP"
echo "  fail2ban:               $CFG_F2B"
echo "  unattended-upgrades:    $CFG_UU"
read -rp "Proceed with installation? [Y/n]: " PROCEED
PROCEED="${PROCEED:-Y}"
[[ "$PROCEED" =~ ^[Yy]$ ]] || abort "Aborted."

# ---------- System prep ----------
export DEBIAN_FRONTEND=noninteractive
apt-get update -y
apt-get upgrade -y
apt-get install -y ca-certificates curl gnupg lsb-release ufw wget jq openssl libnss3-tools

if [[ "$CFG_UU" =~ ^[Yy]$ ]]; then
  apt-get install -y unattended-upgrades apt-listchanges
  dpkg-reconfigure -f noninteractive unattended-upgrades || true
fi

if [[ "$CFG_F2B" =~ ^[Yy]$ ]]; then
  apt-get install -y fail2ban
  systemctl enable --now fail2ban
fi

# Swap
if [[ "$CFG_SWAP" =~ ^[Yy]$ ]]; then
  if ! swapon --show | grep -q '^'; then
    echo "Creating 2G swapfile..."
    fallocate -l 2G /swapfile || dd if=/dev/zero of=/swapfile bs=1M count=2048
    chmod 600 /swapfile
    mkswap /swapfile
    swapon /swapfile
    grep -q "/swapfile" /etc/fstab || echo "/swapfile none swap sw 0 0" >> /etc/fstab
  else
    echo "Swap already present; skipping."
  fi
fi

# UFW - Include NATS port 4222
if [[ "$CFG_UFW" =~ ^[Yy]$ ]]; then
  ufw allow "${SSH_PORT}/tcp" || true
  ufw allow 80/tcp || true
  ufw allow 443/tcp || true
  ufw allow 4222/tcp || true  # NATS TLS port
  yes | ufw enable || true
  ufw status
fi

# ---------- Docker & Compose ----------
if ! command_exists docker; then
  echo "Installing Docker..."
  curl -fsSL https://get.docker.com | sh
fi
apt-get install -y docker-compose-plugin
systemctl enable --now docker

# ---------- Install mkcert ----------
echo "Installing mkcert for TLS certificates..."
if ! command_exists mkcert; then
  curl -L https://github.com/FiloSottile/mkcert/releases/latest/download/mkcert-linux-amd64 -o /usr/local/bin/mkcert
  chmod +x /usr/local/bin/mkcert
  mkcert -install
fi

# ---------- App directory ----------
APP_DIR="/opt/voidloop"
mkdir -p "$APP_DIR"
cd "$APP_DIR"

# ---------- NATS TLS Configuration ----------
echo "Setting up NATS with TLS..."
mkdir -p "$APP_DIR/nats-config"
cd "$APP_DIR/nats-config"

# Generate certificates for NATS
mkcert -cert-file nats-server-cert.pem -key-file nats-server-key.pem "nats.${DOMAIN}"

# Copy the root CA
cp "$(mkcert -CAROOT)/rootCA.pem" "$APP_DIR/nats-config/rootCA.pem"

# Create NATS configuration with TLS and authentication
cat > "$APP_DIR/nats-config/nats-server.conf" <<EOF
listen: 0.0.0.0:4222

authorization: {
    users: [
        {user: "matchmaker", password: "${NATS_MATCHMAKER_PASSWORD}"},
        {user: "matchmaker-httpd", password: "${NATS_MATCHMAKER_HTTPD_PASSWORD}"},
        {user: "gameserver", password: "${NATS_GAMESERVER_PASSWORD}"},
        {user: "lobby", password: "${NATS_LOBBY_PASSWORD}"}
    ]
}

tls {
  cert_file: "/config/nats-server-cert.pem"
  key_file:  "/config/nats-server-key.pem"
}

jetstream {
    store_dir: /data
    max_memory_store: 104857600
    max_file_store:   104857600
}
EOF

cd "$APP_DIR"

# ---------- .env ----------
cat > .env <<EOF
DOMAIN=${DOMAIN}
EMAIL=${EMAIL}
EDGEGAP_API_KEY=${EDGEGAP_API_KEY}
LIGHTYEAR_PROTOCOL_ID=${LIGHTYEAR_PROTOCOL_ID}
LIGHTYEAR_PRIVATE_KEY=${LIGHTYEAR_PRIVATE_KEY}
MAX_PLAYERS=${MAX_PLAYERS}

# NATS Configuration
NATS_HOST=nats.${DOMAIN}
NATS_MATCHMAKER_USER=matchmaker
NATS_MATCHMAKER_PASSWORD=${NATS_MATCHMAKER_PASSWORD}
NATS_MATCHMAKER_HTTPD_USER=matchmaker-httpd
NATS_MATCHMAKER_HTTPD_PASSWORD=${NATS_MATCHMAKER_HTTPD_PASSWORD}
NATS_GAMESERVER_USER=gameserver
NATS_GAMESERVER_PASSWORD=${NATS_GAMESERVER_PASSWORD}
NATS_LOBBY_USER=lobby
NATS_LOBBY_PASSWORD=${NATS_LOBBY_PASSWORD}

# Database
DB_PASSWORD=${DB_PASSWORD}
JWT_SECRET=${JWT_SECRET}

# Docker Hub images
CLIENT_IMAGE=${CLIENT_IMAGE}
MM_HTTPD_IMAGE=${MM_HTTPD_IMAGE}
MM_IMAGE=${MM_IMAGE}
LOBBY_IMAGE=${LOBBY_IMAGE}
EOF

# ---------- Caddyfile ----------
cat > Caddyfile <<EOF
{
  email \${EMAIL}
}

\${DOMAIN} {
  encode zstd gzip

  header {
    Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
    X-Frame-Options "SAMEORIGIN"
    X-Content-Type-Options "nosniff"
    Cross-Origin-Embedder-Policy "require-corp"
    Cross-Origin-Opener-Policy "same-origin"
  }

  # Client static files
  handle {
    reverse_proxy client:80
  }

  # Matchmaker WebSocket and HTTP endpoints
  handle /matchmaker/* {
    reverse_proxy matchmaker-httpd:3000
  }

  # Webhook endpoints - direct routing to lobby service
  handle /hook/* {
    reverse_proxy lobby:3001
  }

  # Health check endpoint
  handle /health {
    respond "OK" 200
  }
}
EOF

# ---------- docker-compose.yml with TLS ----------
cat > docker-compose.yml <<EOF
version: "3.9"

networks:
  public:
  internal:
    internal: true

volumes:
  caddy_data:
  caddy_config:
  pg_data:
  nats_data:

services:
  caddy:
    image: caddy:2.8-alpine
    restart: unless-stopped
    depends_on:
      - client
      - matchmaker-httpd
      - lobby
    ports:
      - "80:80"
      - "443:443"
    environment:
      DOMAIN: \${DOMAIN}
      EMAIL: \${EMAIL}
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    networks: [public, internal]
    read_only: true
    tmpfs: ["/tmp"]
    security_opt:
      - "no-new-privileges:true"
    logging:
      driver: "json-file"
      options: { max-size: "10m", max-file: "3" }

  client:
    image: \${CLIENT_IMAGE}
    restart: unless-stopped
    environment:
      - MATCHMAKER_URL=wss://\${DOMAIN}/matchmaker/ws
    networks: [internal]
    read_only: false
    tmpfs: ["/tmp"]
    logging:
      driver: "json-file"
      options: { max-size: "10m", max-file: "3" }

  nats:
    image: nats:2.10-alpine
    restart: unless-stopped
    # The nats image already runs nats-server; this passes the config path
    command: ["--config", "/config/nats-server.conf"]
    networks: [internal]
    ports:
      - "4222:4222"
    volumes:
      - /opt/voidloop/nats-config:/config:ro
      - nats_data:/data
    read_only: true
    tmpfs:
      - /tmp
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

  matchmaker-httpd:
    image: \${MM_HTTPD_IMAGE}
    restart: unless-stopped
    environment:
      - NATS_HOST=nats
      - NATS_USER=matchmaker-httpd
      - NATS_PASSWORD=\${NATS_MATCHMAKER_HTTPD_PASSWORD}
      - NATS_CA=/nats-ca/rootCA.pem
    volumes:
      - /opt/voidloop/nats-config/rootCA.pem:/nats-ca/rootCA.pem:ro
    command: >
      --bind 0.0.0.0:3000
      --cors https://\${DOMAIN}
      --player-limit \${MAX_PLAYERS}
      --fake-ip 127.0.0.1
    networks: [internal, public]
    read_only: true
    tmpfs: ["/tmp"]
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://localhost:3000/ || exit 1"]
      interval: 15s
      timeout: 3s
      retries: 5
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

  matchmaker:
    image: \${MM_IMAGE}
    restart: unless-stopped
    environment:
      - NATS_HOST=nats
      - NATS_USER=matchmaker
      - NATS_PASSWORD=\${NATS_MATCHMAKER_PASSWORD}
      - NATS_CA=/nats-ca/rootCA.pem
      - EDGEGAP_API_KEY=\${EDGEGAP_API_KEY}
    volumes:
      - /opt/voidloop/nats-config/rootCA.pem:/nats-ca/rootCA.pem:ro
    command: >
      --app-name voidloop-quest
      --app-version v0.0.17
      --lightyear-protocol-id \${LIGHTYEAR_PROTOCOL_ID}
      --lightyear-private-key "\${LIGHTYEAR_PRIVATE_KEY}"
      --player-limit \${MAX_PLAYERS}
    networks: [internal, public]
    read_only: true
    tmpfs: ["/tmp"]
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

  postgres:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=lobby
      - POSTGRES_PASSWORD=\${DB_PASSWORD}
      - POSTGRES_DB=voidloop_lobby
    volumes:
      - pg_data:/var/lib/postgresql/data
    networks: [internal]
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U lobby -d voidloop_lobby || exit 1"]
      interval: 10s
      timeout: 3s
      retries: 10
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

  lobby:
    image: \${LOBBY_IMAGE}
    restart: unless-stopped
    environment:
      - DATABASE_URL=postgres://lobby:\${DB_PASSWORD}@postgres:5432/voidloop_lobby
      - JWT_SECRET=\${JWT_SECRET}
      - NATS_HOST=nats
      - NATS_USER=lobby
      - NATS_PASSWORD=\${NATS_LOBBY_PASSWORD}
      - NATS_CA=/nats-ca/rootCA.pem
    volumes:
      - /opt/voidloop/nats-config/rootCA.pem:/nats-ca/rootCA.pem:ro
    depends_on:
      - postgres
      - nats
    networks: [internal, public]
    read_only: true
    tmpfs: ["/tmp"]
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://localhost:3001/ || exit 1"]
      interval: 15s
      timeout: 3s
      retries: 5
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
EOF

# ---------- Postgres backup script & cron ----------
mkdir -p "$APP_DIR/backups"
cat > "$APP_DIR/backup_pg.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
TS=$(date +%F_%H-%M)
DIR="/opt/voidloop/backups"
mkdir -p "$DIR"
docker compose -f /opt/voidloop/docker-compose.yml exec -T postgres pg_dump -U lobby voidloop_lobby | gzip > "$DIR/pg_${TS}.sql.gz"
find "$DIR" -type f -name 'pg_*.sql.gz' -mtime +7 -delete
EOF
chmod +x "$APP_DIR/backup_pg.sh"

# Add cron (once)
crontab -l 2>/dev/null | grep -q "/opt/voidloop/backup_pg.sh" || (
  (crontab -l 2>/dev/null; echo "15 3 * * * /opt/voidloop/backup_pg.sh >/dev/null 2>&1") | crontab -
)

# ---------- Create gameserver configuration notes ----------
cat > "$APP_DIR/GAMESERVER_CONFIG.md" <<EOF
# Gameserver Configuration

Your gameserver Docker image needs to connect to NATS with TLS. Include these environment variables:

\`\`\`bash
NATS_HOST=nats.${DOMAIN}
NATS_USER=gameserver
NATS_PASSWORD=${NATS_GAMESERVER_PASSWORD}
NATS_CA=/path/to/rootCA.pem  # Must include the rootCA.pem in your gameserver image
\`\`\`

The rootCA.pem file is located at: $APP_DIR/nats-config/rootCA.pem

## For Edgegap Deployment

1. Include the rootCA.pem in your gameserver Docker image
2. Set the NATS environment variables in your Edgegap deployment configuration
3. Ensure your gameserver can reach nats.${DOMAIN}:4222

## Testing NATS Connection

Install NATS CLI:
\`\`\`bash
curl -L https://github.com/nats-io/natscli/releases/latest/download/nats-cli-linux-amd64 -o /usr/local/bin/nats
chmod +x /usr/local/bin/nats
\`\`\`

Test connection:
\`\`\`bash
nats context save --server "tls://nats.${DOMAIN}:4222" \\
  --user "gameserver" --password "${NATS_GAMESERVER_PASSWORD}" \\
  --tlsca "$APP_DIR/nats-config/rootCA.pem" --select bevygap
nats server check connection
\`\`\`
EOF

# ---------- Create troubleshooting guide ----------
cat > "$APP_DIR/TROUBLESHOOTING.md" <<EOF
# Troubleshooting Guide

## Client Error: "ServerMultiMessageSender::metadata failed validation"

This error indicates the client is trying to connect to a game server that isn't properly initialized.

### Common Causes:
1. No game server is running
2. Matchmaker can't spawn servers (missing EDGEGAP_API_KEY)
3. Network connectivity issues between client and server
4. Protocol mismatch between client and server

### Solutions:

1. **Check if matchmaker is running:**
   \`\`\`bash
   docker compose -f /opt/voidloop/docker-compose.yml logs matchmaker --tail=50
   \`\`\`

2. **Verify EDGEGAP_API_KEY is set:**
   \`\`\`bash
   grep EDGEGAP_API_KEY /opt/voidloop/.env
   \`\`\`
   If empty, update it and restart:
   \`\`\`bash
   nano /opt/voidloop/.env
   docker compose -f /opt/voidloop/docker-compose.yml restart matchmaker
   \`\`\`

3. **Check LIGHTYEAR_PRIVATE_KEY format:**
   Must be exactly 32 comma-separated bytes, e.g.:
   \`1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32\`

4. **Verify NATS connectivity:**
   \`\`\`bash
   docker compose -f /opt/voidloop/docker-compose.yml logs nats --tail=20
   \`\`\`

5. **Check if game servers can connect:**
   Look for connection logs from gameservers in NATS logs

## DNS Issues

Ensure both DNS records are configured:
- ${DOMAIN} -> VPS IP
- nats.${DOMAIN} -> VPS IP

Test with:
\`\`\`bash
nslookup ${DOMAIN}
nslookup nats.${DOMAIN}
\`\`\`

## Certificate Issues

If TLS connections fail:
1. Verify certificates exist:
   \`\`\`bash
   ls -la /opt/voidloop/nats-config/
   \`\`\`

2. Regenerate if needed:
   \`\`\`bash
   cd /opt/voidloop/nats-config
   mkcert -cert-file nats-server-cert.pem -key-file nats-server-key.pem nats.${DOMAIN}
   docker compose -f /opt/voidloop/docker-compose.yml restart nats
   \`\`\`

## Service Health Checks

\`\`\`bash
# Check all services
docker compose -f /opt/voidloop/docker-compose.yml ps

# Check individual service logs
docker compose -f /opt/voidloop/docker-compose.yml logs [service-name] --tail=50

# Test endpoints
curl -I https://${DOMAIN}/health
curl -I https://${DOMAIN}/lobby/health
curl -I https://${DOMAIN}/matchmaker/health
\`\`\`
EOF

# ---------- Start stack ----------
echo "Pulling images..."
docker compose pull || true
echo "Starting services..."
docker compose up -d

# Wait for services to start
echo "Waiting for services to initialize..."
sleep 10

# Check service status
docker compose ps

echo
echo "=== Deployment complete ==="
echo "Location: $APP_DIR"
echo "Files:"
echo "  - $APP_DIR/.env (contains all passwords and configuration)"
echo "  - $APP_DIR/Caddyfile"
echo "  - $APP_DIR/docker-compose.yml"
echo "  - $APP_DIR/nats-config/ (TLS certificates and config)"
echo "  - $APP_DIR/backup_pg.sh (cron 03:15 daily, keeps 7 days)"
echo "  - $APP_DIR/GAMESERVER_CONFIG.md (gameserver connection info)"
echo "  - $APP_DIR/TROUBLESHOOTING.md (common issues and solutions)"
echo
echo "NATS TLS Configuration:"
echo "  - Public endpoint: nats.${DOMAIN}:4222 (TLS)"
echo "  - Root CA: $APP_DIR/nats-config/rootCA.pem"
echo "  - Gameserver password saved in GAMESERVER_CONFIG.md"
echo
echo "Check status:"
echo "  docker compose -f $APP_DIR/docker-compose.yml ps"
echo "  docker compose -f $APP_DIR/docker-compose.yml logs -f"
echo
echo "Test endpoints (once DNS is live and TLS issued):"
echo "  curl -I https://${DOMAIN}/health"
echo "  curl -I https://${DOMAIN}/lobby/health"
echo "  curl -I https://${DOMAIN}/matchmaker/health"
echo
echo "Test NATS TLS connection:"
echo "  See instructions in $APP_DIR/GAMESERVER_CONFIG.md"
echo
if [ -z "${EDGEGAP_API_KEY:-}" ] || [ -z "${LIGHTYEAR_PRIVATE_KEY:-}" ]; then
  echo "⚠️  IMPORTANT: Update missing secrets in $APP_DIR/.env and restart:"
  echo "  nano $APP_DIR/.env"
  echo "  docker compose -f $APP_DIR/docker-compose.yml up -d"
fi
echo
echo "📖 If you encounter the client error, see: $APP_DIR/TROUBLESHOOTING.md"
