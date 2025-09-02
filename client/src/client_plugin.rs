use bevy::prelude::*;

#[cfg(feature = "bevygap")]
use bevygap_client_plugin::{BevygapClientPlugin, prelude::BevygapConnectExt};

use leafwing_input_manager::prelude::*;

use shared::{Player, PlayerActions, PlayerColor, PlayerTransform, Platform, SharedPlugin};
use crate::screens::{LobbyPlugin, AppState};

#[derive(Resource, Default)]
struct FloorSpawned(bool);

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        // Get matchmaker URL from browser location
        let matchmaker_url = get_matchmaker_url();
        info!("Matchmaker url: {}", matchmaker_url);
        
        // Basic Bevy plugins
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Voidloop Quest".to_string(),
                canvas: Some("#game".to_string()),
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }));
        
        // Input plugin
        app.add_plugins(InputManagerPlugin::<PlayerActions>::default());
        
        // Networking plugins (only if bevygap feature is enabled)
        #[cfg(feature = "bevygap")]
        app.add_plugins(BevygapClientPlugin);
        
        // Lobby system - handles 4-player lobby UI and matchmaking
        app.add_plugins(LobbyPlugin);
        
        // Shared game logic
        app.add_plugins(SharedPlugin);
        
        // Camera setup - needed for both Lobby UI and InGame
        app.add_systems(Startup, setup_camera);
        
        // Game setup systems (only run when in game)
        app.add_systems(OnEnter(AppState::InGame), setup_game);
        app.add_systems(Update, (
            spawn_player_visual,
            spawn_platform_visual,
            update_player_visual,
            handle_player_spawn,
        ).run_if(in_state(AppState::InGame)));
        app.insert_resource(FloorSpawned::default());

        // Remove auto-connect - now handled by lobby UI
        // app.add_systems(Startup, |mut commands: Commands| {
        //     commands.bevygap_connect_client();
        // });
        
        // ==== CUSTOM CLIENT SYSTEMS AREA - Add your client-specific systems here ====
        // Example: UI, effects, client-side predictions, etc.
        // app.add_systems(Update, your_custom_client_system);
        // ==== END CUSTOM CLIENT SYSTEMS AREA ====
    }
}

fn get_matchmaker_url() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        info!("Creating matchmaker url from window.location");
        let window = web_sys::window().expect("no global `window` exists");
        let location = window.location();
        let protocol = if location.protocol().unwrap() == "https:" {
            "wss"
        } else {
            "ws"
        };
        let host = location.host().unwrap();
        format!("{}://{}/matchmaker/ws", protocol, host)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "ws://localhost:3000/matchmaker/ws".to_string()
    }
}

fn setup_camera(mut commands: Commands) {
    // Spawn 2D camera with UI support - needed for both lobby UI and game
    commands.spawn((
        Camera2d::default(),
        Camera {
            clear_color: ClearColorConfig::Default,
            ..default()
        },
    ));
}

fn setup_game(mut commands: Commands) {
    // Spawn some platforms for the level (only when entering game)
    spawn_platforms(&mut commands);
}

fn spawn_platforms(commands: &mut Commands) {
    // Floor is handled in the physics system at y = -200
    
    // Add some floating platforms
    let platform_positions = vec![
        Vec3::new(-200.0, -100.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(200.0, -50.0, 0.0),
        Vec3::new(-300.0, 50.0, 0.0),
        Vec3::new(300.0, 100.0, 0.0),
    ];
    
    for pos in platform_positions {
        commands.spawn((
            Platform,
            Transform::from_translation(pos),
        ));
    }
}

// Handle when a new player spawns (including local player)
fn handle_player_spawn(
    mut commands: Commands,
    new_players: Query<Entity, Added<Player>>,
) {
    for entity in new_players.iter() {
        // Add input handling for local player
        // TODO: Determine if this is the local player
        commands.entity(entity).insert((
            InputMap::<PlayerActions>::default()
                .with(PlayerActions::MoveLeft, KeyCode::KeyA)
                .with(PlayerActions::MoveLeft, KeyCode::ArrowLeft)
                .with(PlayerActions::MoveRight, KeyCode::KeyD)
                .with(PlayerActions::MoveRight, KeyCode::ArrowRight)
                .with(PlayerActions::Jump, KeyCode::Space)
                .with(PlayerActions::Jump, KeyCode::KeyW)
                .with(PlayerActions::Jump, KeyCode::ArrowUp),
            ActionState::<PlayerActions>::default(),
        ));
        
        info!("Player spawned with controls: A/D or Arrow keys to move, Space/W to jump");
    }
}

// Spawn visual representation for players
fn spawn_player_visual(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    new_players: Query<(Entity, &PlayerColor, &PlayerTransform), Added<Player>>,
) {
    for (entity, color, transform) in new_players.iter() {
        commands.entity(entity).insert((
            Mesh2d(meshes.add(Rectangle::new(30.0, 30.0))),
            MeshMaterial2d(materials.add(color.color)),
            Transform::from_translation(transform.translation),
        ));
    }
}

// Spawn visual representation for platforms
fn spawn_platform_visual(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    new_platforms: Query<(Entity, &Transform), Added<Platform>>,
    mut floor_spawned: ResMut<FloorSpawned>,
) {
    for (entity, transform) in new_platforms.iter() {
        commands.entity(entity).insert((
            Mesh2d(meshes.add(Rectangle::new(200.0, 20.0))),
            MeshMaterial2d(materials.add(Color::srgb(0.3, 0.3, 0.3))),
            *transform,
        ));
    }
    
    // Also spawn a visual floor (only once on startup)
    if !floor_spawned.0 {
        floor_spawned.0 = true;
        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(1000.0, 20.0))),
            MeshMaterial2d(materials.add(Color::srgb(0.2, 0.2, 0.2))),
            Transform::from_xyz(0.0, -210.0, 0.0),
        ));
    }
}

// Update player visual position
fn update_player_visual(
    mut query: Query<(&mut Transform, &PlayerTransform), (With<Player>, Changed<PlayerTransform>)>,
) {
    for (mut transform, player_transform) in query.iter_mut() {
        transform.translation = player_transform.translation;
    }
}

// ==== CUSTOM CLIENT RENDERING AREA - Add your visual effects and UI here ====
// Example: Particle effects, UI overlays, animations, etc.
// 
// fn my_custom_render_system(
//     // your queries and resources
// ) {
//     // your rendering logic
// }
// ==== END CUSTOM CLIENT RENDERING AREA ====
