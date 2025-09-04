use serde::{Deserialize, Serialize};
use std::env;

#[cfg(feature = "matchmaker")]
use axum::{
    extract::State,
    http::{Method, StatusCode},
    response::Json,
    routing::post,
    Router,
};

#[cfg(feature = "matchmaker")]
use edgegap_async::{
    apis::{configuration::Configuration, lobbies_api},
    models::{LobbyCreatePayload, LobbyDeployPayload},
};

#[cfg(feature = "matchmaker")]
use tower_http::cors::{Any, CorsLayer};

// Shared request/response structures (should match client)
#[derive(Serialize, Deserialize, Debug)]
pub struct MatchmakingRequest {
    pub game_mode: String,
    pub player_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MatchmakingResponse {
    pub success: bool,
    pub lobby_name: Option<String>,
    pub server_url: Option<String>,
    pub error_message: Option<String>,
}

#[cfg(feature = "matchmaker")]
#[derive(Clone)]
struct AppState {
    edgegap_config: Configuration,
}

#[cfg(feature = "matchmaker")]
async fn handle_matchmaking(
    State(state): State<AppState>,
    Json(request): Json<MatchmakingRequest>,
) -> Result<Json<MatchmakingResponse>, StatusCode> {
    println!("🔍 Matchmaking request: {:?}", request);

    // Generate unique lobby name using random ID
    let random_id = rand::random::<u32>() % 90000 + 10000;
    let lobby_name = format!("voidloop-{}-{}", request.game_mode, random_id);

    println!("🔧 Creating Edgegap lobby: {}", lobby_name);

    // Create lobby
    let payload = LobbyCreatePayload::new(lobby_name.clone());
    let create_result = lobbies_api::lobby_create(&state.edgegap_config, payload).await;

    match create_result {
        Ok(create_response) => {
            println!("✅ Lobby created: {}", create_response.name);

            // Deploy the lobby (this starts the game server)
            let deploy_payload = LobbyDeployPayload {
                name: create_response.name.clone(),
            };
            let deploy_result = lobbies_api::lobby_deploy(&state.edgegap_config, deploy_payload).await;

            match deploy_result {
                Ok(deploy_response) => {
                    println!("🚀 Lobby deployed successfully!");
                    println!("📍 Server URL: {}", deploy_response.url);
                    println!("📊 Status: {}", deploy_response.status);
                    
                    Ok(Json(MatchmakingResponse {
                        success: true,
                        lobby_name: Some(create_response.name),
                        server_url: Some(deploy_response.url),
                        error_message: None,
                    }))
                }
                Err(e) => {
                    let error_msg = format!("Failed to deploy lobby: {:?}", e);
                    eprintln!("❌ {}", error_msg);
                    Ok(Json(MatchmakingResponse {
                        success: false,
                        lobby_name: None,
                        server_url: None,
                        error_message: Some(error_msg),
                    }))
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to create lobby: {:?}", e);
            eprintln!("❌ {}", error_msg);
            Ok(Json(MatchmakingResponse {
                success: false,
                lobby_name: None,
                server_url: None,
                error_message: Some(error_msg),
            }))
        }
    }
}

#[cfg(feature = "matchmaker")]
pub async fn run_matchmaker_service() -> Result<(), Box<dyn std::error::Error>> {
    // Get Edgegap configuration from environment
    let edgegap_base_url = env::var("EDGEGAP_BASE_URL")
        .unwrap_or_else(|_| "https://api.edgegap.com".to_string());
    let edgegap_token = env::var("EDGEGAP_TOKEN")
        .map_err(|_| "EDGEGAP_TOKEN environment variable is required")?;

    // Configure Edgegap API client
    let mut edgegap_config = Configuration::default();
    edgegap_config.base_path = edgegap_base_url;
    edgegap_config.api_key = Some(edgegap_async::apis::configuration::ApiKey {
        prefix: Some("Bearer".into()),
        key: edgegap_token,
    });

    let app_state = AppState { edgegap_config };

    // Setup CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any)
        .allow_origin(Any);

    // Build our application with routes
    let app = Router::new()
        .route("/api/matchmaking", post(handle_matchmaking))
        .layer(cors)
        .with_state(app_state);

    // Bind to port
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    println!("🚀 Matchmaker service listening on {}", addr);
    println!("🔐 Edgegap token configured securely server-side");
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[cfg(not(feature = "matchmaker"))]
pub fn run_matchmaker_service() {
    eprintln!("❌ Matchmaker service not compiled - enable 'matchmaker' feature");
    std::process::exit(1);
}