# Build stage
FROM rust:1.81-slim AS builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY shared ./shared
COPY lobby-service ./lobby-service

# Build the lobby service
RUN cargo build --release -p lobby-service

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN adduser --disabled-password --gecos "" --uid 1000 lobby

# Copy binary
COPY --from=builder /app/target/release/lobby-service /usr/local/bin/lobby-service

USER lobby

EXPOSE 3001

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

CMD ["lobby-service"]