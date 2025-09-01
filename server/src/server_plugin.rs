use bevy::prelude::*;
use bevygap_server_plugin::prelude::BevygapServerPlugin;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;

use shared::{PlayerActions, Platform, SharedPlugin};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        // Minimal Bevy plugins for server
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_secs_f32(1.0 / 60.0), // 60 FPS server tick rate
        ));
        
        // Input plugin
        app.add_plugins(InputManagerPlugin::<PlayerActions>::default());
        
        // Networking
        app.add_plugins(BevygapServerPlugin);
        
        // Shared game logic
        app.add_plugins(SharedPlugin);
        
        // Server-specific systems
        app.add_systems(Startup, setup_world);
        // Connection handling is managed by bevygap
        // app.add_systems(Update, (
        //     handle_new_connections,
        //     handle_disconnections,
        // ));
        
        // ==== CUSTOM SERVER SYSTEMS AREA - Add your server-specific logic here ====
        // Example: Game rules, scoring, AI, matchmaking logic, etc.
        // app.add_systems(Update, your_custom_server_system);
        // ==== END CUSTOM SERVER SYSTEMS AREA ====
    }
}

fn setup_world(mut commands: Commands) {
    info!("Setting up game world...");
    
    // Spawn platforms (these will be replicated to clients)
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
            ReplicationTarget::<Client>::default(),
        ));
    }
    
    info!("World setup complete with {} platforms", 5);
}

// Connection handling functions - commented out until we determine correct event types
/*
fn handle_new_connections(
    mut commands: Commands,
    mut connections: EventReader<ServerConnectEvent>,
) {
    for event in connections.read() {
        let client_id = event.client_id;
        info!("New player connected: {:?}", client_id);
        
        // Random spawn position
        let mut rng = rand::thread_rng();
        let spawn_x = rng.gen_range(-300.0..300.0);
        let spawn_pos = Vec3::new(spawn_x, 100.0, 0.0);
        
        // Random player color
        let color = Color::srgb(
            rng.gen_range(0.3..1.0),
            rng.gen_range(0.3..1.0),
            rng.gen_range(0.3..1.0),
        );
        
        // Spawn player entity
        let player_entity = commands.spawn((
            Player::default(),
            PlayerTransform {
                translation: spawn_pos,
            },
            PlayerColor { color },
            InputMap::<PlayerActions>::default(),
            ActionState::<PlayerActions>::default(),
            ReplicationTarget::<Client>::default(),
            client_id,
        )).id();
        
        info!("Spawned player entity {:?} for client {:?} at position {:?}", 
              player_entity, client_id, spawn_pos);
    }
}

fn handle_disconnections(
    mut commands: Commands,
    mut disconnections: EventReader<ServerDisconnectEvent>,
    players: Query<(Entity, &Client)>,
) {
    for event in disconnections.read() {
        let client_id = event.client_id;
        info!("Player disconnected: {:?}", client_id);
        
        // Remove the player entity
        for (entity, player_client_id) in players.iter() {
            if *player_client_id == client_id {
                commands.entity(entity).despawn();
                info!("Removed player entity {:?} for disconnected client {:?}", 
                      entity, client_id);
            }
        }
    }
}
*/

// ==== CUSTOM SERVER LOGIC AREA - Add your game rules and server logic here ====
// Example: Scoring system, match management, AI opponents, etc.
// 
// fn my_custom_server_logic(
//     // your queries and resources
// ) {
//     // your server logic
// }
// ==== END CUSTOM SERVER LOGIC AREA ====
