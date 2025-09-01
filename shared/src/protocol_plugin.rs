use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

// Simple player actions for platformer
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Actionlike)]
pub enum PlayerActions {
    MoveLeft,
    MoveRight,
    Jump,
}

// Player component with position and velocity
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Player {
    pub velocity: Vec2,
    pub grounded: bool,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            velocity: Vec2::ZERO,
            grounded: false,
        }
    }
}

// Transform component for position
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerTransform {
    pub translation: Vec3,
}

impl Default for PlayerTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
        }
    }
}

// Platform component for level geometry
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Platform;

// Color component for visual representation
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerColor {
    pub color: Color,
}

impl Default for PlayerColor {
    fn default() -> Self {
        Self {
            color: Color::srgb(0.0, 0.5, 1.0),
        }
    }
}

// Channel for reliable messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Channel1;

// Protocol Plugin
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Register components for replication
        app.register_component::<Player>()
            .add_prediction(PredictionMode::Full)
            .add_interpolation(InterpolationMode::Full);
            
        app.register_component::<PlayerTransform>()
            .add_prediction(PredictionMode::Full)
            .add_interpolation(InterpolationMode::Full);
            
        app.register_component::<PlayerColor>()
            .add_prediction(PredictionMode::Once);
            
        app.register_component::<Platform>()
            .add_prediction(PredictionMode::Once);
        
        // Register channel
        app.add_channel::<Channel1>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        });
        
        // Register input
        app.add_plugins(lightyear::prelude::input::leafwing::InputPlugin::<PlayerActions>::default());
    }
}

// Helper function to create protocol
pub fn protocol() -> ProtocolPlugin {
    ProtocolPlugin
}

// ==== CUSTOM GAME CODE AREA - Add your game-specific components and systems here ====
// Example: Add new components, systems, or game mechanics below this line
// 
// #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
// pub struct MyCustomComponent {
//     pub value: f32,
// }
//
// Remember to register new components in the ProtocolPlugin build() method above!
// ==== END CUSTOM GAME CODE AREA ====
