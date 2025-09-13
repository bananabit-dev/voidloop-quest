use bevy::prelude::*;

#[cfg(feature = "bevygap")]
use bevygap_client_plugin::BevygapClientPlugin;

#[cfg(feature = "bevygap")]
use bevygap_client_plugin::prelude::BevygapClientConfig;

use leafwing_input_manager::prelude::*;

use crate::screens::{AppState, LobbyPlugin};
use shared::{
    Platform, Player, PlayerActions, PlayerAnimationState, PlayerColor, PlayerId, PlayerTransform,
    SharedPlugin,
};

// Resource to hold the Vey character model handle and animation graph
#[derive(Resource)]
struct VeyModel {
    scene: Handle<Scene>,
    animation_graph: Handle<AnimationGraph>,
    idle_node: AnimationNodeIndex,
    running_node: AnimationNodeIndex,
    t_pose_node: AnimationNodeIndex,
    jumping_node: AnimationNodeIndex,
}

// Component to mark entities that need the Vey model spawned
#[derive(Component)]
struct VeyModelToLoad;

// Component to mark the actual 3D model entity with animation player
#[derive(Component)]
struct VeyModelEntity {
    animation_player: Entity,
}

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

        #[cfg(feature = "bevygap")]
        {
            app.insert_resource(BevygapClientConfig {
                matchmaker_url,
                fake_client_ip: None,
                game_name: "voidloop-quest".into(),
                game_version: "0".into(),
            });
        }

        // Camera setup - needed for both Lobby UI and InGame
        app.add_systems(Startup, (setup_camera, load_vey_model));

        // Game setup systems (only run when in game)
        app.add_systems(OnEnter(AppState::InGame), setup_game);
        app.add_systems(
            Update,
            (
                spawn_player_visual,
                spawn_platform_visual,
                update_player_visual,
                handle_player_spawn,
                update_vey_model_transform,
                update_vey_model_animations, // Renamed and updated system
            )
                .run_if(in_state(AppState::InGame)),
        );
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
    // Spawn 3D camera positioned for 2.5D platformer view
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 500.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.1, 0.2, 0.3)),
            ..default()
        },
    ));

    // Add basic lighting for 3D models
    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, -0.5, 0.0)),
    ));
}

fn load_vey_model(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Load the main scene from the GLB file
    let vey_scene = asset_server.load("vey.glb#Scene0");

    // Load the four animations from the GLB file
    // Based on the comment, animations are: t-pose, jumping, idle, and running
    let idle_animation = asset_server.load("vey.glb#Animation0"); // idle
    let t_pose_animation = asset_server.load("vey.glb#Animation1"); // t-pose
    let running_animation = asset_server.load("vey.glb#Animation2"); // running
    let jumping_animation = asset_server.load("vey.glb#Animation3"); // jumping

    // Create animation graph with the loaded animations
    let mut animation_graph = AnimationGraph::new();
    let idle_node = animation_graph.add_clip(idle_animation, 1.0, animation_graph.root);
    let t_pose_node = animation_graph.add_clip(t_pose_animation, 1.0, animation_graph.root);
    let running_node = animation_graph.add_clip(running_animation, 1.0, animation_graph.root);
    let jumping_node = animation_graph.add_clip(jumping_animation, 1.0, animation_graph.root);

    let animation_graph_handle = animation_graphs.add(animation_graph);

    // Create VeyModel resource with proper animation nodes
    commands.insert_resource(VeyModel {
        scene: vey_scene,
        animation_graph: animation_graph_handle,
        idle_node,
        running_node,
        t_pose_node,
        jumping_node,
    });

    info!("🎭 Loading Vey character model with four animations: idle (Animation0), t-pose (Animation1), running (Animation2), jumping (Animation3)");
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
        commands.spawn((Platform, Transform::from_translation(pos)));
    }
}

// Handle when a new player spawns (add input to local player only)
fn handle_player_spawn(
    mut commands: Commands,
    new_players: Query<(Entity, &PlayerId), Added<Player>>,
) {
    for (entity, player_id) in new_players.iter() {
        // Only add input handling to the first player (local player)
        if player_id.id == 0 {
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

            info!("🎮 Local player {} spawned with controls: A/D or Arrow keys to move, Space/W to jump", player_id.id);
        } else {
            info!("👤 Remote player {} spawned", player_id.id);
        }
    }
}

// Spawn 3D visual representation for players using Vey model
fn spawn_player_visual(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vey_model: Option<Res<VeyModel>>,
    new_players: Query<(Entity, &PlayerColor, &PlayerTransform, &PlayerId), Added<Player>>,
) {
    for (entity, color, transform, player_id) in new_players.iter() {
        // Determine color variation for multiplayer
        let final_color = if player_id.id == 0 {
            color.color // Original color for player 1
        } else {
            // Lighter variant for player 2+
            Color::srgb(
                (color.color.to_srgba().red + 0.3).min(1.0),
                (color.color.to_srgba().green + 0.3).min(1.0),
                (color.color.to_srgba().blue + 0.3).min(1.0),
            )
        };

        let model_entity = if let Some(vey_model) = &vey_model {
            // Use GLB model if available
            let animation_player = commands
                .spawn((
                    AnimationPlayer::default(),
                    AnimationGraphHandle(vey_model.animation_graph.clone()),
                ))
                .id();

            let model_entity = commands
                .spawn((
                    SceneRoot(vey_model.scene.clone()),
                    Transform::from_scale(Vec3::splat(50.0)), // Scale the model appropriately
                    VeyModelEntity { animation_player },
                ))
                .add_child(animation_player)
                .id();

            model_entity
        } else {
            // Fallback: Create a simple geometric character (capsule)
            info!(
                "🎭 GLTF model not loaded, using geometric fallback for player {}",
                player_id.id
            );
            commands
                .spawn((
                    Mesh3d(meshes.add(Capsule3d::new(8.0, 40.0))), // Simple capsule character
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: final_color,
                        metallic: 0.1,
                        perceptual_roughness: 0.9,
                        ..default()
                    })),
                    Transform::from_translation(Vec3::new(0.0, 20.0, 0.0)), // Center the capsule
                    VeyModelEntity {
                        animation_player: Entity::PLACEHOLDER,
                    }, // No animation for fallback
                ))
                .id()
        };

        // Set up the player entity with 3D transform
        commands
            .entity(entity)
            .insert((
                Transform::from_translation(transform.translation),
                Visibility::default(),
                VeyModelToLoad,
            ))
            .add_child(model_entity);

        if vey_model.is_some() {
            info!(
                "🎭 Spawned 3D Vey GLB model for player {} with animation support",
                player_id.id
            );
        } else {
            info!(
                "🎭 Spawned fallback geometric character for player {}",
                player_id.id
            );
        }
    }
}

// Spawn 3D visual representation for platforms
fn spawn_platform_visual(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    new_platforms: Query<(Entity, &Transform), Added<Platform>>,
    mut floor_spawned: ResMut<FloorSpawned>,
) {
    for (entity, transform) in new_platforms.iter() {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Cuboid::new(200.0, 20.0, 50.0))), // 3D cuboid for platforms
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.3, 0.3),
                ..default()
            })),
            *transform,
        ));
    }

    // Also spawn a visual floor (only once on startup)
    if !floor_spawned.0 {
        floor_spawned.0 = true;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1000.0, 20.0, 100.0))), // 3D floor
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.2, 0.2),
                ..default()
            })),
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

// Update Vey model transform to follow player
fn update_vey_model_transform(
    player_query: Query<(Entity, &PlayerTransform), (With<Player>, Changed<PlayerTransform>)>,
    mut model_query: Query<&mut Transform, (With<VeyModelEntity>, Without<Player>)>,
    children_query: Query<&Children>,
) {
    for (player_entity, _player_pos) in player_query.iter() {
        // Find children that are Vey models
        if let Ok(children) = children_query.get(player_entity) {
            for child in children.iter() {
                if let Ok(mut model_transform) = model_query.get_mut(child) {
                    // Update the model position to match player
                    model_transform.translation = Vec3::ZERO; // Relative to parent
                }
            }
        }
    }
}

// Update Vey model animations based on player state
fn update_vey_model_animations(
    player_query: Query<
        (&PlayerAnimationState, &Children),
        (With<Player>, Changed<PlayerAnimationState>),
    >,
    model_query: Query<&VeyModelEntity, Without<Player>>,
    mut animation_players: Query<&mut AnimationPlayer>,
    mut transforms: Query<&mut Transform, With<VeyModelEntity>>,
    vey_model: Option<Res<VeyModel>>,
) {
    let Some(vey_model) = vey_model else {
        return;
    };

    for (anim_state, children) in player_query.iter() {
        for child in children.iter() {
            if let Ok(vey_entity) = model_query.get(child) {
                // Update model orientation (mirroring for left/right movement)
                if let Ok(mut model_transform) = transforms.get_mut(child) {
                    let scale_x = if anim_state.facing_left { -50.0 } else { 50.0 };
                    model_transform.scale = Vec3::new(scale_x, 50.0, 50.0);
                }

                // Update animations
                if vey_entity.animation_player != Entity::PLACEHOLDER {
                    if let Ok(mut animation_player) =
                        animation_players.get_mut(vey_entity.animation_player)
                    {
                        // Determine which animation to play based on state
                        let (target_node, anim_name) = if anim_state.is_jumping {
                            (vey_model.jumping_node, "jumping") // Use jumping animation for jumping/falling
                        } else if anim_state.is_moving {
                            (vey_model.running_node, "running")
                        } else {
                            (vey_model.idle_node, "idle")
                        };

                        // Play the animation
                        animation_player.play(target_node).repeat();
                        info!("🎬 Playing {} animation for player", anim_name);
                    }
                }
            }
        }
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
