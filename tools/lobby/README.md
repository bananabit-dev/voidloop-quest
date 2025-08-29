# Lobby tool

Small non-blocking CLI to manage Edgegap lobbies using bevygap's async client (`edgegap_async`).

## Build

```
cargo build -p lobby
```

## Usage

Set env vars (or pass flags):

- `EDGEGAP_BASE_URL` e.g. `https://api.edgegap.com`
- `EDGEGAP_TOKEN` your API token

Then run commands, e.g.:

```
# Create
cargo run -p lobby -- --base-url $EDGEGAP_BASE_URL --token $EDGEGAP_TOKEN create my-lobby

# Get
cargo run -p lobby -- get my-lobby

# Deploy
cargo run -p lobby -- deploy my-lobby

# List
cargo run -p lobby -- list

# Terminate
cargo run -p lobby -- terminate my-lobby

# Delete
cargo run -p lobby -- delete my-lobby
```

All API calls are async via reqwest; no threads are blocked.

