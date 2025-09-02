use bevy::prelude::*;
use bevygap_server_plugin::prelude::BevygapServerPlugin;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;
use rand::Rng;

use shared::{Player, PlayerActions, PlayerColor, PlayerTransform, Platform, SharedPlugin};

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
        
        // TODO: Connection handling will be implemented when bevygap events are available
        // For now, we'll use the shared plugin which handles the platformer physics
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
            Replicate::default(),
        ));
    }
    
    info!("World setup complete with {} platforms", 5);
}

// TODO: Connection handling functions for player management
// Will be implemented when bevygap connection events are properly integrated
/*
fn handle_new_connections(
    mut commands: Commands,
    // Need to determine correct event type for bevygap
) {
    // Player spawning logic will go here
}

fn handle_disconnections(
    mut commands: Commands,
    // Need to determine correct event type for bevygap
) {
    // Player removal logic will go here
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
