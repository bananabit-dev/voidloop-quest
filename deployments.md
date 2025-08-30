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
