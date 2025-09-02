# NOTE:
# Build from the monorepo root so Docker context contains both voidloop-quest and bevygap:
#   docker build -f voidloop-quest/client/Dockerfile .
# or
#   docker build -f work/voidloop-quest/client/Dockerfile .
# (Do NOT use voidloop-quest/client as the build context because the workspace depends on ../bevygap.)

# ---------- Base toolchain image ----------
FROM rust:1-slim AS base

# Install required build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-chef for caching dependencies
RUN cargo install cargo-chef --locked

# Install wasm-bindgen-cli with specific version for compatibility
RUN cargo install wasm-bindgen-cli --version 0.2.100 --locked

# Add WASM target
RUN rustup target add wasm32-unknown-unknown

# Set environment for WASM builds
ENV CARGO_BUILD_TARGET=wasm32-unknown-unknown
ENV TARGET_CC=/usr/bin/clang
ENV RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""

# ---------- Planner stage ----------
FROM base AS planner
WORKDIR /app

# Copy the entire workspace
COPY . .

# Prepare the recipe for dependency caching
RUN cargo chef prepare --recipe-path /app/recipe.json

# ---------- Builder stage ----------
FROM base AS builder
WORKDIR /app

# Copy recipe and cook dependencies (this is cached unless dependencies change)
COPY --from=planner /app/recipe.json /app/recipe.json

# Build dependencies only (cached layer)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --target wasm32-unknown-unknown \
    --package voidloop-quest-client \
    --recipe-path /app/recipe.json

# Copy the entire workspace
COPY . .

# Build the actual application
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target \
    cargo build --release --target wasm32-unknown-unknown --package voidloop-quest-client && \
    wasm-bindgen --no-typescript --target web \
        --out-dir /app/www \
        --out-name "voidloop-quest" \
        /app/target/wasm32-unknown-unknown/release/voidloop-quest-client.wasm

# Create the index.html and other web assets
RUN cat > /app/www/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Voidloop Quest</title>
    <style>
        body {
            margin: 0;
            padding: 0;
            overflow: hidden;
            background: #000;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
        }
        canvas {
            display: block;
            max-width: 100%;
            max-height: 100vh;
        }
        #loading {
            position: absolute;
            color: white;
            font-family: monospace;
            font-size: 24px;
        }
    </style>
</head>
<body>
    <div id="loading">Loading...</div>
    <canvas id="game"></canvas>
    <script type="module">
        import init from './voidloop-quest.js';
        
        async function run() {
            await init();
            document.getElementById('loading').style.display = 'none';
        }
        
        run().catch(console.error);
    </script>
</body>
</html>
EOF

# ---------- Final stage: Nginx server ----------
FROM nginx:alpine AS runtime

# Add non-root user for security
RUN adduser -D -H -s /sbin/nologin nginx-user

# Copy built WASM files
COPY --from=builder /app/www /usr/share/nginx/html/

# Copy game assets
COPY --from=builder /app/assets /usr/share/nginx/html/assets/

# Create nginx config with proper headers for WASM
RUN cat > /etc/nginx/conf.d/default.conf << 'EOF'
server {
    listen 80;
    server_name _;
    root /usr/share/nginx/html;
    index index.html;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # CORS headers for WASM
    add_header Cross-Origin-Embedder-Policy "require-corp" always;
    add_header Cross-Origin-Opener-Policy "same-origin" always;

    # Proper MIME types
    location ~ \.wasm$ {
        add_header Content-Type application/wasm;
    }

    location / {
        try_files $uri $uri/ /index.html;
    }

    # Health check endpoint
    location /health {
        access_log off;
        return 200 "healthy\n";
        add_header Content-Type text/plain;
    }
}
EOF

# Run nginx as non-root
RUN chown -R nginx-user:nginx-user /usr/share/nginx/html \
    && chown -R nginx-user:nginx-user /var/cache/nginx \
    && chown -R nginx-user:nginx-user /var/log/nginx \
    && touch /var/run/nginx.pid \
    && chown nginx-user:nginx-user /var/run/nginx.pid

USER nginx-user

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
