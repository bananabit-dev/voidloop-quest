#!/usr/bin/env bash
# Voidloop Quest one-command VPS setup (Ubuntu/Debian)
# - Installs Docker & Compose
# - Configures UFW firewall and a 2G swapfile
# - Generates secrets and config
# - Creates Caddy reverse proxy (auto TLS), NATS (auth), Matchmaker, Lobby (with Postgres), Client
# - Starts the stack with Docker Compose
#
# Requirements:
# - Run as root (sudo -i) on Ubuntu 20.04+/Debian 11+
# - DNS A/AAAA of your domain points to this VPS
#
# Note:
# - You must provide container image names that are accessible (public or docker login beforehand)
# - If your images are private, run `docker login` before executing this script

set -euo pipefail

# ---------- Helpers ----------
abort() { echo "Error: $*" >&2; exit 1; }
require_root() { [ "$(id -u)" -eq 0 ] || abort "Run as root (sudo -i)"; }
command_exists() { command -v "$1" >/dev/null 2>&1; }
gen_hex() { openssl rand -hex "${1:-32}"; }
detect_ssh_port() { sshd -T 2>/dev/null | awk '/^port /{print $2; exit}' || echo "22"; }

require_root

echo "=== Voidloop Quest VPS setup ==="

# ---------- Collect configuration ----------
read -rp "Domain (e.g., voidloop.quest): " DOMAIN
[ -n "${DOMAIN:-}" ] || abort "Domain is required"

read -rp "Admin email for TLS (Let's Encrypt): " EMAIL
[ -n "${EMAIL:-}" ] || abort "Email is required"

echo "Container images (use public images or login before)."
read -rp "Client image (e.g., ghcr.io/yourorg/voidloop-quest-client:latest): " CLIENT_IMAGE
[ -n "${CLIENT_IMAGE:-}" ] || abort "Client image is required"

read -rp "Matchmaker HTTPD image (e.g., ghcr.io/yourorg/bevygap_matchmaker_httpd:latest): " MM_HTTPD_IMAGE
[ -n "${MM_HTTPD_IMAGE:-}" ] || abort "Matchmaker HTTPD image is required"

read -rp "Matchmaker image (e.g., ghcr.io/yourorg/bevygap_matchmaker:latest): " MM_IMAGE
[ -n "${MM_IMAGE:-}" ] || abort "Matchmaker image is required"

read -rp "Lobby image (e.g., ghcr.io/yourorg/bevygap_lobby:latest): " LOBBY_IMAGE
[ -n "${LOBBY_IMAGE:-}" ] || abort "Lobby image is required"

read -rp "Max players per session [16]: " MAX_PLAYERS
MAX_PLAYERS="${MAX_PLAYERS:-16}"

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

read -rp "NATS username [matchmaker]: " NATS_USER
NATS_USER="${NATS_USER:-matchmaker}"

read -rp "Generate strong NATS password? [Y/n]: " GEN_NATS_PASS
GEN_NATS_PASS="${GEN_NATS_PASS:-Y}"
if [[ "$GEN_NATS_PASS" =~ ^[Yy]$ ]]; then
  NATS_PASSWORD="$(gen_hex 32)"
  echo "Generated NATS password."
else
  read -rsp "NATS password (input hidden): " NATS_PASSWORD
  echo
  [ -n "${NATS_PASSWORD:-}" ] || abort "NATS password required"
fi

DB_PASSWORD="$(gen_hex 32)"
JWT_SECRET="$(gen_hex 64)"

read -rp "Configure UFW firewall (allow SSH/80/443) and enable? [Y/n]: " CFG_UFW
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
echo "  Email:                  $EMAIL"
echo "  Client image:           $CLIENT_IMAGE"
echo "  MM HTTPD image:         $MM_HTTPD_IMAGE"
echo "  MM image:               $MM_IMAGE"
echo "  Lobby image:            $LOBBY_IMAGE"
echo "  Max players:            $MAX_PLAYERS"
echo "  Lightyear Protocol ID:  $LIGHTYEAR_PROTOCOL_ID"
echo "  Lightyear Private Key:  ${LIGHTYEAR_PRIVATE_KEY:+[SET]}${LIGHTYEAR_PRIVATE_KEY:+" (hidden)"}"
echo "  Edgegap API Key:        ${EDGEGAP_API_KEY:+[SET]}${EDGEGAP_API_KEY:+" (hidden)"}"
echo "  NATS user:              $NATS_USER"
echo "  NATS password:          [generated or provided]"
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
apt-get install -y ca-certificates curl gnupg lsb-release ufw wget jq

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

# UFW
if [[ "$CFG_UFW" =~ ^[Yy]$ ]]; then
  ufw allow "${SSH_PORT}/tcp" || true
  ufw allow 80/tcp || true
  ufw allow 443/tcp || true
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

# ---------- App directory ----------
APP_DIR="/opt/voidloop"
mkdir -p "$APP_DIR"
cd "$APP_DIR"

# ---------- .env ----------
cat > .env <<EOF
DOMAIN=${DOMAIN}
EMAIL=${EMAIL}
EDGEGAP_API_KEY=${EDGEGAP_API_KEY}
LIGHTYEAR_PROTOCOL_ID=${LIGHTYEAR_PROTOCOL_ID}
LIGHTYEAR_PRIVATE_KEY=${LIGHTYEAR_PRIVATE_KEY}
MAX_PLAYERS=${MAX_PLAYERS}
NATS_USER=${NATS_USER}
NATS_PASSWORD=${NATS_PASSWORD}
DB_PASSWORD=${DB_PASSWORD}
JWT_SECRET=${JWT_SECRET}

CLIENT_IMAGE=${CLIENT_IMAGE}
MM_HTTPD_IMAGE=${MM_HTTPD_IMAGE}
MM_IMAGE=${MM_IMAGE}
LOBBY_IMAGE=${LOBBY_IMAGE}
EOF

# ---------- Caddyfile ----------
cat > Caddyfile <<EOF
{
  email ${EMAIL}
}

${DOMAIN} {
  encode zstd gzip

  header {
    Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
    X-Frame-Options "SAMEORIGIN"
    X-Content-Type-Options "nosniff"
    Cross-Origin-Embedder-Policy "require-corp"
    Cross-Origin-Opener-Policy "same-origin"
  }

  handle_path /matchmaker* {
    reverse_proxy matchmaker-httpd:3000
  }

  handle_path /lobby* {
    reverse_proxy lobby:3001
  }

  reverse_proxy client:80
}
EOF

# ---------- docker-compose.yml ----------
cat > docker-compose.yml <<'EOF'
version: "3.9"

networks:
  public:
  internal:
    internal: true

volumes:
  caddy_data:
  caddy_config:
  pg_data:

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
      DOMAIN: ${DOMAIN}
      EMAIL: ${EMAIL}
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
    image: ${CLIENT_IMAGE}
    restart: unless-stopped
    environment:
      - MATCHMAKER_URL=wss://${DOMAIN}/matchmaker/ws
    networks: [internal]
    read_only: true
    tmpfs: ["/tmp"]
    logging:
      driver: "json-file"
      options: { max-size: "10m", max-file: "3" }

  nats:
    image: nats:2.10-alpine
    restart: unless-stopped
    command: ["-p", "4222", "-user", "${NATS_USER}", "-pass", "${NATS_PASSWORD}"]
    networks: [internal]
    read_only: true
    tmpfs: ["/tmp"]
    logging:
      driver: "json-file"
      options: { max-size: "10m", max-file: "3" }

  matchmaker-httpd:
    image: ${MM_HTTPD_IMAGE}
    restart: unless-stopped
    environment:
      - NATS_HOST=nats
      - NATS_USER=${NATS_USER}
      - NATS_PASS=${NATS_PASSWORD}
    command: >
      --bind 0.0.0.0:3000
      --cors https://${DOMAIN}
      --player-limit ${MAX_PLAYERS}
      --fake-ip 127.0.0.1
    networks: [internal]
    read_only: true
    tmpfs: ["/tmp"]
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://localhost:3000/health || exit 1"]
      interval: 15s
      timeout: 3s
      retries: 5
    logging:
      driver: "json-file"
      options: { max-size: "10m", max-file: "3" }

  matchmaker:
    image: ${MM_IMAGE}
    restart: unless-stopped
    environment:
      - NATS_HOST=nats
      - NATS_USER=${NATS_USER}
      - NATS_PASS=${NATS_PASSWORD}
      - EDGEGAP_API_KEY=${EDGEGAP_API_KEY}
    command: >
      --app-name voidloop-quest
      --app-version 1
      --lightyear-protocol-id ${LIGHTYEAR_PROTOCOL_ID}
      --lightyear-private-key "${LIGHTYEAR_PRIVATE_KEY}"
      --player-limit ${MAX_PLAYERS}
    networks: [internal]
    read_only: true
    tmpfs: ["/tmp"]
    logging:
      driver: "json-file"
      options: { max-size: "10m", max-file: "3" }

  postgres:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=lobby
      - POSTGRES_PASSWORD=${DB_PASSWORD}
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
      options: { max-size: "10m", max-file: "3" }

  lobby:
    image: ${LOBBY_IMAGE}
    restart: unless-stopped
    environment:
      - DATABASE_URL=postgres://lobby:${DB_PASSWORD}@postgres:5432/voidloop_lobby
      - JWT_SECRET=${JWT_SECRET}
      - NATS_HOST=nats
      - NATS_USER=${NATS_USER}
      - NATS_PASS=${NATS_PASSWORD}
    depends_on:
      - postgres
      - nats
    networks: [internal]
    read_only: true
    tmpfs: ["/tmp"]
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://localhost:3001/health || exit 1"]
      interval: 15s
      timeout: 3s
      retries: 5
    logging:
      driver: "json-file"
      options: { max-size: "10m", max-file: "3" }
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

# ---------- Start stack ----------
echo "Pulling images..."
docker compose pull || true
echo "Starting services..."
docker compose up -d

echo
echo "=== Deployment complete ==="
echo "Location: $APP_DIR"
echo "Files:"
echo "  - $APP_DIR/.env"
echo "  - $APP_DIR/Caddyfile"
echo "  - $APP_DIR/docker-compose.yml"
echo "  - $APP_DIR/backup_pg.sh (cron 03:15 daily, keeps 7 days)"
echo
echo "Check status:"
echo "  docker compose -f $APP_DIR/docker-compose.yml ps"
echo "  docker compose -f $APP_DIR/docker-compose.yml logs -f caddy"
echo
echo "Test endpoints (once DNS is live and TLS issued):"
echo "  curl -I https://${DOMAIN}/health"
echo "  curl -I https://${DOMAIN}/lobby/health"
echo "  # WebSocket:"
echo "  # npm i -g wscat && wscat -c wss://${DOMAIN}/matchmaker/ws"
echo
if [ -z "${EDGEGAP_API_KEY:-}" ] || [ -z "${LIGHTYEAR_PRIVATE_KEY:-}" ]; then
  echo "Important: Update missing secrets in $APP_DIR/.env and restart:"
  echo "  nano $APP_DIR/.env"
  echo "  docker compose -f $APP_DIR/docker-compose.yml up -d"
fi