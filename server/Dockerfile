# NOTE:
# Build from the monorepo root so Docker context contains both voidloop-quest and bevygap:
#   docker build -f voidloop-quest/server/Dockerfile .
# or
#   docker build -f work/voidloop-quest/server/Dockerfile .
# (Do NOT use voidloop-quest/server as the build context because the workspace depends on ../bevygap.)

# ---------- Base toolchain image ----------
FROM rust:1-slim AS base

# Install required build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libudev-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-chef for dependency caching
RUN cargo install cargo-chef --locked

# ---------- Planner stage ----------
FROM base AS planner
WORKDIR /app

# Copy the entire workspace
COPY . .

# Prepare recipe for caching
RUN cargo chef prepare --recipe-path /app/recipe.json

# ---------- Builder stage ----------
FROM base AS builder
WORKDIR /app

# Copy recipe and cook dependencies (cached)
COPY --from=planner /app/recipe.json /app/recipe.json

# Build dependencies only
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --package server --recipe-path /app/recipe.json

# Copy the entire workspace
COPY . .

# Build the server binary
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target \
    cd voidloop-quest && \
    cargo build --release --package server && \
    # Copy the binary to a known location
    cp /app/target/release/server /app/server-binary

# ---------- Runtime stage ----------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1001 -s /bin/bash gameserver

# Create app directory
WORKDIR /app

# Copy binary and assets
COPY --from=builder --chown=gameserver:gameserver /app/server-binary /app/server
COPY --from=builder --chown=gameserver:gameserver /app/voidloop-quest/assets /app/assets

# Make binary executable
RUN chmod +x /app/server

# Create directory for game data/logs
RUN mkdir -p /app/data /app/logs && \
    chown -R gameserver:gameserver /app/data /app/logs

# Switch to non-root user
USER gameserver

# Environment variables for configuration
ENV RUST_LOG=info
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=6420
ENV TRANSPORT_PORT=6421
ENV LOBBY_URL=""
ENV LOBBY_API_KEY=""

# Expose ports
# Game server UDP port
EXPOSE 6420/udp
# WebTransport port
EXPOSE 6421/tcp
# Metrics/health check port
EXPOSE 9090/tcp

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:9090/health || exit 1

# Entry point with signal handling
ENTRYPOINT ["/app/server"]

# Default command-line arguments (can be overridden)
CMD ["--host", "0.0.0.0", "--port", "6420", "--transport-port", "6421"]
