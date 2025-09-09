# Lobby tool

Small non-blocking CLI to manage Edgegap lobbies using bevygap's async client (`edgegap_async`).

⚠️ **IMPORTANT**: For lobby deployment to spawn game servers, you must provide `app_name` and `app_version` parameters that correspond to your Edgegap application.

## Build

```
cargo build -p lobby
```

## Usage

Set env vars (or pass flags):

- `EDGEGAP_BASE_URL` e.g. `https://api.edgegap.com`
- `EDGEGAP_TOKEN` your API token
- `EDGEGAP_APP_NAME` your Edgegap application name (required for deployment)
- `EDGEGAP_APP_VERSION` your Edgegap application version (required for deployment)

Then run commands, e.g.:

```
# Create (basic)
cargo run -p lobby -- --base-url $EDGEGAP_BASE_URL --token $EDGEGAP_TOKEN create my-lobby

# Create with app configuration (recommended)
cargo run -p lobby -- --app-name voidloop-quest-server --app-version 1.0.0 create my-lobby

# Deploy with app configuration (REQUIRED for game server spawning)
cargo run -p lobby -- --app-name voidloop-quest-server --app-version 1.0.0 deploy my-lobby

# Deploy without app configuration (will show warning)
cargo run -p lobby -- deploy my-lobby

# Get / List
cargo run -p lobby -- get my-lobby
cargo run -p lobby -- list

# Terminate / Delete
cargo run -p lobby -- terminate my-lobby
cargo run -p lobby -- delete my-lobby
```

### Environment Variables

For convenience, set these environment variables:

```bash
export EDGEGAP_BASE_URL="https://api.edgegap.com"
export EDGEGAP_TOKEN="your-edgegap-api-token"
export EDGEGAP_APP_NAME="voidloop-quest-server"
export EDGEGAP_APP_VERSION="1.0.0"

# Then you can run commands without flags:
cargo run -p lobby -- deploy my-lobby
```

### How It Works

1. **Create**: Creates a named lobby on Edgegap
2. **Deploy**: Deploys the lobby as a game server instance
   - With `app_name` and `app_version`: Attempts to spawn the specified application
   - Without app configuration: May not spawn a game server (shows warning)
3. **Get/List**: Shows lobby status and server URLs when deployed
4. **Terminate/Delete**: Stops and removes lobbies

All API calls are async via reqwest; no threads are blocked.

## Troubleshooting

### Lobby deployment doesn't spawn game servers

**Problem**: `lobby deploy` succeeds but no game server is created.

**Solution**: Ensure you provide `--app-name` and `--app-version` parameters:

```bash
cargo run -p lobby -- --app-name your-app-name --app-version your-app-version deploy my-lobby
```

Or set environment variables:
```bash
export EDGEGAP_APP_NAME="your-app-name"
export EDGEGAP_APP_VERSION="your-app-version"
```

### Application not found

**Problem**: Edgegap returns 404 or "application not found" error.

**Solution**: 
1. Verify your app name and version exist in the Edgegap dashboard
2. Ensure the application version is active
3. Check that your API token has permissions for the application

