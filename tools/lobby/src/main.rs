use anyhow::Result;
use clap::{Parser, Subcommand};
use edgegap_async::apis::{configuration::Configuration, lobbies_api};
use edgegap_async::models::{LobbyCreatePayload, LobbyDeployPayload, LobbyTerminatePayload};

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
    cfg.base_path = cli.base_url;
    cfg.api_key = Some(edgegap_async::apis::configuration::ApiKey {
        prefix: Some("Bearer".into()),
        key: cli.token,
    });

    match cli.command {
        Commands::Create { name } => {
            let payload = LobbyCreatePayload::new(name);
            let res = lobbies_api::lobby_create(&cfg, payload).await?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::Deploy { name } => {
            let payload = LobbyDeployPayload { name };
            let res = lobbies_api::lobby_deploy(&cfg, payload).await?;
            println!("{}", serde_json::to_string_pretty(&res)?);
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
