use anyhow::Result;
use clap::{Parser, Subcommand};
use edgegap_async::apis::{configuration::Configuration, lobbies_api};
use edgegap_async::models::{LobbyCreatePayload, LobbyDeployPayload, LobbyTerminatePayload};
use serde::{Deserialize, Serialize};

/// Enhanced lobby create payload with app configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnhancedLobbyCreatePayload {
    /// Name of the lobby
    pub name: String,
    /// Application name to deploy (if supported by API)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// Application version to deploy (if supported by API)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
}

/// Enhanced lobby deploy payload with app configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnhancedLobbyDeployPayload {
    /// Name of the lobby
    pub name: String,
    /// Application name to deploy (if supported by API)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// Application version to deploy (if supported by API)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
}

#[derive(Parser, Debug)]
#[command(
    name = "lobby",
    about = "Edgegap lobby helper using bevygap's async client"
)]
struct Cli {
    /// Base URL for Edgegap API (e.g. https://api.edgegap.com)
    #[arg(long, env = "EDGEGAP_BASE_URL")]
    base_url: String,

    /// API token for Edgegap (sent as Authorization: Bearer <token>)
    #[arg(long, env = "EDGEGAP_TOKEN")]
    token: String,

    /// App name for Edgegap deployment (required for lobby deployment)
    #[arg(long, env = "EDGEGAP_APP_NAME")]
    app_name: Option<String>,

    /// App version for Edgegap deployment (required for lobby deployment)
    #[arg(long, env = "EDGEGAP_APP_VERSION")]
    app_version: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new lobby with the given name
    Create { name: String },
    /// Deploy a lobby by name
    Deploy { name: String },
    /// Terminate a lobby by name
    Terminate { name: String },
    /// Delete a lobby by name
    Delete { name: String },
    /// Get lobby details by name
    Get { name: String },
    /// List all lobbies
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut cfg = Configuration::default();
    cfg.base_path = cli.base_url.clone();
    cfg.api_key = Some(edgegap_async::apis::configuration::ApiKey {
        prefix: Some("Bearer".into()),
        key: cli.token.clone(),
    });

    match cli.command {
        Commands::Create { name } => {
            // For create, we'll try to use enhanced payload if app info is provided,
            // otherwise fall back to basic payload
            if let (Some(app_name), Some(app_version)) = (&cli.app_name, &cli.app_version) {
                println!("Creating lobby '{}' with app: {} v{}", name, app_name, app_version);
                // Try enhanced create with app info - this may or may not be supported by the API
                let enhanced_payload = EnhancedLobbyCreatePayload {
                    name: name.clone(),
                    app_name: Some(app_name.clone()),
                    app_version: Some(app_version.clone()),
                };
                
                // We'll use a custom API call since we can't modify the edgegap_async models
                let client = reqwest::Client::new();
                let url = format!("{}/v1/lobbies", cfg.base_path);
                let response = client
                    .post(&url)
                    .header("authorization", format!("Bearer {}", cli.token))
                    .json(&enhanced_payload)
                    .send()
                    .await?;
                
                if response.status().is_success() {
                    let text = response.text().await?;
                    println!("{}", text);
                } else {
                    eprintln!("Enhanced create failed (status: {}), falling back to basic create...", response.status());
                    // Fall back to basic create
                    let payload = LobbyCreatePayload::new(name);
                    let res = lobbies_api::lobby_create(&cfg, payload).await?;
                    println!("{}", serde_json::to_string_pretty(&res)?);
                }
            } else {
                let payload = LobbyCreatePayload::new(name);
                let res = lobbies_api::lobby_create(&cfg, payload).await?;
                println!("{}", serde_json::to_string_pretty(&res)?);
            }
        }
        Commands::Deploy { name } => {
            // For deploy, app_name and app_version are strongly recommended
            if let (Some(app_name), Some(app_version)) = (&cli.app_name, &cli.app_version) {
                println!("Deploying lobby '{}' with app: {} v{}", name, app_name, app_version);
                
                // Try enhanced deploy with app info
                let enhanced_payload = EnhancedLobbyDeployPayload {
                    name: name.clone(),
                    app_name: Some(app_name.clone()),
                    app_version: Some(app_version.clone()),
                };
                
                let client = reqwest::Client::new();
                let url = format!("{}/v1/lobbies:deploy", cfg.base_path);
                let response = client
                    .post(&url)
                    .header("authorization", format!("Bearer {}", cli.token))
                    .json(&enhanced_payload)
                    .send()
                    .await?;
                
                if response.status().is_success() {
                    let text = response.text().await?;
                    println!("{}", text);
                } else {
                    eprintln!("Enhanced deploy failed (status: {}), falling back to basic deploy...", response.status());
                    // Fall back to basic deploy
                    let payload = LobbyDeployPayload { name };
                    let res = lobbies_api::lobby_deploy(&cfg, payload).await?;
                    println!("{}", serde_json::to_string_pretty(&res)?);
                }
            } else {
                eprintln!("⚠️  WARNING: Deploying lobby without app_name and app_version.");
                eprintln!("   This may not spawn a game server. Consider setting:");
                eprintln!("   --app-name <your-app-name> --app-version <your-app-version>");
                eprintln!("   Or use environment variables EDGEGAP_APP_NAME and EDGEGAP_APP_VERSION");
                eprintln!("");
                
                let payload = LobbyDeployPayload { name };
                let res = lobbies_api::lobby_deploy(&cfg, payload).await?;
                println!("{}", serde_json::to_string_pretty(&res)?);
            }
        }
        Commands::Terminate { name } => {
            let payload = LobbyTerminatePayload { name };
            let res = lobbies_api::lobby_terminate(&cfg, payload).await?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::Delete { name } => {
            let res = lobbies_api::lobby_delete(&cfg, &name).await?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::Get { name } => {
            let res = lobbies_api::lobby_get(&cfg, &name).await?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::List => {
            let res = lobbies_api::lobby_list(&cfg).await?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
    }

    Ok(())
}
