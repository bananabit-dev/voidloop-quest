use bevy::prelude::*;
use client_plugin::ClientPlugin;

mod client_plugin;

fn main() {
    info!("🎮 Simple Platformer Client starting...");
    info!("🔐 Using BevyGap for matchmaking and connection");
    
    App::new()
        .add_plugins(ClientPlugin)
        .run();
}
