# Build stage
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

# Build the lobby server
RUN cargo build --release -p lobby-server

# Runtime stage
FROM debian:bookworm-slim

# Install required system dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /app/target/release/lobby-server /usr/local/bin/lobby-server

# Create non-root user
RUN useradd -m -u 1000 lobbyuser
USER lobbyuser

EXPOSE 3001

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3001/lobby/health || exit 1

CMD ["lobby-server", "--host", "0.0.0.0", "--port", "3001"]