# Deployments

This document describes how to build and run the Docker images for the voidloop-quest project, and references any CI workflows if present.

Project root: ./work/voidloop-quest

Dockerfiles

1) Server Dockerfile
- Path: work/voidloop-quest/server/Dockerfile
- Purpose: Builds a native Linux server binary and packages it into a minimal Distroless image.
- Multi-stage overview:
  - base: metabrew/rust-chef-sccache:latest with sccache configured
  - planner: cargo-chef prepare --bin server
  - builder: cargo-chef cook --bin server, then cargo build --release -p server
  - final: gcr.io/distroless/cc-debian12, copies assets and the server binary
- Exposed ports: 6420/udp
- Entrypoint: ./server

Build example

```
# From repository root
DOCKER_BUILDKIT=1 docker build -f work/voidloop-quest/server/Dockerfile -t voidloop-quest-server:latest work/voidloop-quest
```

Run example

```
docker run --rm -p 6420:6420/udp voidloop-quest-server:latest
```

2) Client Dockerfile
- Path: work/voidloop-quest/client/Dockerfile
- Purpose: Builds the WebAssembly client and serves it via nginx.
- Multi-stage overview:
  - base: metabrew/rust-chef-sccache:0.0.6 with wasm target and RUSTFLAGS for web_sys_unstable_apis
  - planner: cargo-chef prepare --bin client
  - builder: cargo-chef cook, cargo build --release -p client (target wasm32-unknown-unknown), wasm-bindgen to /app/www
  - final: nginx:mainline-alpine, copies www and assets into /usr/share/nginx/html
- Exposed ports: 80/tcp

Build example

```
# From repository root
DOCKER_BUILDKIT=1 docker build -f work/voidloop-quest/client/Dockerfile -t voidloop-quest-client:latest work/voidloop-quest
```

Run example

```
docker run --rm -p 8080:80 voidloop-quest-client:latest
# Then browse http://localhost:8080/
```

Workflows

- A .github/workflows directory was not found in this repository snapshot, so no CI workflows are documented. If workflows are added later, please update this document with:
  - Workflow file path
  - Trigger (on: push, pull_request, etc.)
  - Jobs summary

Notes

- Ensure Rust toolchains and targets are consistent with your local builds (nightly vs stable) if you mirror these stages locally.
- For production images, pin exact base image digests for reproducibility.
- Consider multi-arch builds via buildx if deploying to mixed architectures.

# Deployment Guide

This guide explains how to deploy the Voidloop Quest demo end-to-end using Bevygap for matchmaking and Edgegap for server orchestration. It also describes how the lobby and player-limit flow works in Voidloop Quest.

Contents
- Components overview
- Prerequisites
- Local development setup
- Production-style setup on Edgegap
- Environment variables and build flags
- How the flow works (client → matchmaker → server)
- Troubleshooting

Components overview
- voidloop-quest
  - client: Bevy + Lightyear game client. Uses bevygap_client_plugin (default feature) to request a session and connect.
  - server: Bevy + Lightyear dedicated game server. Uses bevygap_server_plugin (feature) to publish metadata and accept connections.
- bevygap
  - bevygap_matchmaker_httpd: Axum-based websocket service the client connects to. Proxies requests to NATS and streams progress.
  - bevygap_matchmaker: Async service that talks to Edgegap API to create/link sessions. Generates Lightyear ConnectToken and publishes results back.
  - bevygap_server_plugin: Server-side support for cert digest reporting and server metadata via NATS.
  - bevygap_client_plugin: Client-side flow to request session, receive token + server address, and connect via WebTransport/Netcode.
  - bevy_nfws: Simple websocket client used by bevygap_client_plugin.
- Edgegap
  - Hosts and manages deployments for the dedicated game server container. The matchmaker selects/creates an appropriate session.

Prerequisites
- Rust toolchain (stable)
- Docker (recommended for server image builds)
- NATS server (for local dev: docker run --rm -p 4222:4222 nats:2)
- Edgegap account + API key (for real session creation)

Local development setup
1) Build everything
- Build Bevygap libraries and services:
  - cd work/bevygap
  - cargo build -p bevygap_matchmaker -p bevygap_matchmaker_httpd -p bevygap_server_plugin -p bevygap_client_plugin
- Build Voidloop Quest:
  - cd work/voidloop-quest
  - cargo build -p server -p client

2) Run infrastructure
- Start NATS locally (if not already):
  - docker run --rm -p 4222:4222 nats:2
- Start bevygap_matchmaker_httpd (websocket endpoint):
  - cd work/bevygap
  - cargo run -p bevygap_matchmaker_httpd
  - Defaults to: ws://127.0.0.1:3000/matchmaker/ws
- Start bevygap_matchmaker (Edgegap integration):
  - Requires EDGEGAP_API_KEY env var.
  - cargo run -p bevygap_matchmaker -- \
    --app-name bevygap-spaceships \
    --app-version 1 \
    --lightyear-protocol-id 80085 \
    --lightyear-private-key 'comma,separated,32,bytes,...' \
    [--player-limit 4]
  - Notes:
    - lightyear-private-key must be 32 numbers (0..255) separated by commas.
    - The private key must match the one used by the server for Netcode tokens.

3) Run Voidloop Quest locally
- Client (defaults to bevygap feature enabled):
  - cd work/voidloop-quest
  - MATCHMAKER_URL=ws://127.0.0.1:3000/matchmaker/ws cargo run -p client
- Server (without Edgegap):
  - For a pure local server test, you can run the server directly. In production you’d deploy on Edgegap. For local dev you can still start it:
  - cargo run -p server
  - If you’re using WebTransport in-browser, you’ll need a proper certificate and digest; the standard dev loop assumes servers run in Edgegap.

Production-style setup on Edgegap
1) Build and push server image
- A Dockerfile is included at work/voidloop-quest/server/Dockerfile.
- Build:
  - cd work/voidloop-quest
  - docker build -t your-registry/voidloopquest-server:latest -f server/Dockerfile .
- Push to your registry, and configure Edgegap app/version to use this image.

2) Configure Edgegap application
- In Edgegap, create an application (e.g., bevygap-spaceships) and a version (e.g., 1) that points to the server image.
- Configure port mappings to expose the Lightyear WebTransport port externally.
- Ensure the environment variables in Edgegap app version include:
  - LIGHTYEAR_PRIVATE_KEY (the 32-byte key matching matchmaker) – formatted as comma-separated bytes.
  - LIGHTYEAR_CERTIFICATE_DIGEST (WebTransport cert digest string without colons) – bevygap can record and distribute this if your server publishes it via bevygap_server_plugin.

3) Run the Bevygap services
- Run NATS somewhere reachable by both matchmaker services and server (if using cert digest reporting).
- Run bevygap_matchmaker (with EDGEGAP_API_KEY) and bevygap_matchmaker_httpd – these can be deployed as separate services behind your infra.
- Point your client builds at the HTTPD websocket endpoint:
  - For wasm builds (client/www/index.html), set window.MATCHMAKER_URL.
  - For native builds, set MATCHMAKER_URL environment variable, or bake COMPILE_TIME_MATCHMAKER_URL at compile time.

Environment variables and build flags
- Client (Voidloop Quest)
  - MATCHMAKER_URL: ws(s)://host:port/matchmaker/ws
  - Features:
    - bevygap: enabled by default; uses Bevygap matchmaking flow.
- Server (Voidloop Quest)
  - LIGHTYEAR_PRIVATE_KEY: comma-separated 32-byte array, must match matchmaker
  - LIGHTYEAR_CERTIFICATE_DIGEST: hex digest (no colons) for WebTransport
  - Feature: bevygap (server plugin) if you want metadata reporting and cert digest NATS reporting.
- Bevygap Matchmaker
  - EDGEGAP_API_KEY
  - --app-name, --app-version
  - --lightyear-protocol-id
  - --lightyear-private-key (comma-separated 32 bytes)
  - [--player-limit N]
- Bevygap Matchmaker HTTPD
  - --bind (default 0.0.0.0:3000)
  - --cors (default http://localhost:8000)
  - [--player-limit N] (affects /wannaplay route)
  - --fake-ip (used when clients connect from localhost)


CI/CD Workflows (GitHub Actions)
- Overview
  - This repo contains multiple projects, each with its own GitHub Actions workflows under their respective .github/workflows folders.
  - The two workflows most relevant to deploying Voidloop Quest are in work/voidloop-quest/.github/workflows. Bevygap services also have publish workflows in work/bevygap/.github/workflows.

- Voidloop Quest workflows (work/voidloop-quest/.github/workflows)
  - build-server.yaml
    - Purpose: Build and push the dedicated game server Docker image used by Edgegap.
    - Triggers:
      - push: tags matching v* (e.g., v1.2.3)
      - workflow_dispatch: manual run with an input version (e.g., v1.2.3)
    - Steps (high level):
      - Checkout code
      - Compute Docker tags/labels via docker/metadata-action using the tag or SHA
      - Setup Buildx and log in to Edgegap registry
      - Build and push image using server/Dockerfile
    - Image and tags:
      - Registry: registry.edgegap.com
      - Image name: registry.edgegap.com/${{ secrets.EDGEGAP_IMAGE_NAME }}
      - Tags include the semver (vX.Y.Z), the commit SHA, and latest
    - Required GitHub Secrets:
      - EDGEGAP_IMAGE_NAME: app/image path in Edgegap registry (e.g., org/voidloopquest-server)
      - EDGEGAP_DOCKER_USERNAME, EDGEGAP_DOCKER_PASSWORD: credentials for registry.edgegap.com
    - How to run:
      - Create and push a tag: git tag v1.2.3 && git push origin v1.2.3; or
      - Run manually from the Actions tab and provide version input (v1.2.3)

  - build-wasm.yaml
    - Purpose: Build and push the WebAssembly client Docker image (for static hosting/CDN).
    - Triggers:
      - push: tags matching v*
      - workflow_dispatch with version input
    - Steps (high level):
      - Checkout code
      - Compute Docker tags/labels via docker/metadata-action
      - Setup Buildx and log in to Docker Hub (or your configured registry)
      - Build and push image using client/Dockerfile
    - Image and tags:
      - Env IMAGE_NAME default: metabrew/bevygap-spaceships-wasm (override by editing workflow env)
      - Tags include semver, SHA, and latest
    - Required GitHub Secrets:
      - DOCKERHUB_USERNAME, DOCKERHUB_PASSWORD (or corresponding credentials for your registry)
    - How to run:
      - Push a tag vX.Y.Z or run manually with version input

- Bevygap service workflows (work/bevygap/.github/workflows)
  - publish-matchmaker.yaml
    - Builds ./bevygap_matchmaker/Dockerfile and pushes to Docker Hub as metabrew/bevygap_matchmaker
  - publish-matchmaker-httpd.yaml
    - Builds ./bevygap_matchmaker_httpd/Dockerfile and pushes to Docker Hub as metabrew/bevygap_matchmaker_httpd
  - publish-webhook-sink.yaml
    - Builds ./bevygap_webhook_sink/Dockerfile and pushes to Docker Hub as metabrew/bevygap_webhook_sink
  - Common details:
    - Triggers: tag push v* or manual dispatch with version input
    - Required GitHub Secrets: DOCKER_USERNAME, DOCKER_PASSWORD
    - Tags: semver, SHA, latest

- Lightyear workflows (work/lightyear/.github/workflows)
  - This subproject includes its own upstream workflows (e.g., main.yaml, servers.yaml, wasm.yaml, website.yaml). They are not directly part of the Voidloop Quest deployment path, but are kept for reference and upstream maintenance.

- Cutting a release (end-to-end)
  1) Ensure server and client are ready and, if needed, update any version strings.
  2) Tag and push for the component(s) you want to release:
     - For Voidloop Quest server/client images, push a git tag vX.Y.Z in the repo; corresponding workflows will run.
     - For Bevygap services (if you maintain them here), tag and push in this repo to publish images used by the matchmaker/HTTPD.
  3) Verify the Actions tab shows successful runs and that images exist in the target registries.
  4) Update Edgegap app/version to the new image tag if you don’t track latest.

- Troubleshooting CI/CD
  - Missing secrets or wrong registry credentials → workflows will fail at login; set secrets in GitHub > Settings > Secrets and variables > Actions.
  - Tag format must start with v (e.g., v1.2.3) to auto-trigger.
  - Ensure Dockerfiles exist at paths referenced in each workflow (server/Dockerfile, client/Dockerfile, bevygap_* Dockerfiles).
  - If using a different registry, adjust login action, IMAGE_NAME, and images: fields accordingly.

How it works (client → matchmaker → server)
1) Client UI (Voidloop Quest)
- Lobby screen (client/src/screens/lobby.rs): lets the user pick a desired player limit (1–4) and click Connect. Currently Search/Join are placeholders; Create transitions to Connect.
- Connect screen triggers a ConnectToServerRequest; the Bevygap client plugin handles session setup.

2) Bevygap Client Plugin (client-side)
- On connection to bevygap_matchmaker_httpd via websocket, the client sends RequestSession { game, version, client_ip?, player_limit? }.
- The optional player_limit is carried from the Lobby screen via env var VOIDLOOP_PLAYER_LIMIT.
- The client listens for streamed SessionRequestFeedback messages.
  - Acknowledged → Accepted(session_id) → ProgressReport(…) → SessionReady { token, ip, port, cert_digest }
- On SessionReady, the client configures Lightyear to connect via Netcode/WebTransport using the provided token and digest.

3) Matchmaker HTTPD (axum)
- Terminates websocket and forwards the JSON request to NATS on subject matchmaker.request.{game}.{version}.
- Subscribes to the unique reply inbox and streams messages back to the websocket client.

4) Matchmaker (Edgegap API + NATS)
- Consumes the request, creates (or links to) an Edgegap session (session_post).
- Polls until ready, records certificate digest via NATS KV (if published by the server), builds a Lightyear ConnectToken, and replies with SessionReady.
- Also writes KV entries mapping client_id ↔ session_id for bookkeeping.

5) Server (Lightyear)
- Runs in Edgegap as a dedicated game server, exposes WebTransport + Netcode.
- If compiled with bevygap_server_plugin, publishes server context/metadata, including certificate digest (used by clients to validate WT).

Notes on lobby support in this branch
- Player limit is carried end-to-end. The current matchmaker example always creates a session and does not yet group multiple clients into the same session based on player_limit.
- To build a full lobby experience, extend bevygap_matchmaker to:
  - Maintain a map of candidate sessions per game/version/region/player_limit.
  - Reuse an existing session if it’s not full; only create a new session when needed.
  - Optionally add a list/query API (over HTTPD) and a client-side search/join UI.
- For multi-lobby per dedicated server, use Lightyear Rooms on the server to isolate replication per lobby (see lightyear/examples/lobby for a reference pattern).

Troubleshooting
- “Could not connect to matchmaker websocket”
  - Verify MATCHMAKER_URL is correct and reachable. For TLS, ensure wss:// and proper certificates at HTTPD proxy.
- “Session timeout”
  - Edgegap may take time to pull or start the deployment; verify the app/version is active and pullable. Increase timeouts if needed.
- “Invalid ConnectToken”
  - Ensure LIGHTYEAR_PRIVATE_KEY at the server matches the value provided to the matchmaker. Must be exactly 32 bytes (comma-separated).
- “WebTransport connection failed (TLS cert)”
  - Ensure the server publishes or the client knows the correct certificate digest (no colons). If using Edgegap, bevygap_server_plugin + webhook/kv flow should make it available.

FAQ
- Can I skip bevygap and connect directly?
  - Yes. Disable the bevygap feature in the client and run a local server; see README for direct Netcode settings.
- How do I host the HTTPD behind a proxy?
  - Make sure websockets are forwarded, and set the CORS origin via --cors. If your proxy terminates TLS, use wss:// on the client.
