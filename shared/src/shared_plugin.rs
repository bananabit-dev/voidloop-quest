use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::protocol_plugin::{Platform, Player, PlayerActions, PlayerTransform, PlayerAnimationState};

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                player_movement_system,
                update_animation_state_system,
                apply_gravity_system,
                ground_detection_system,
            )
                .chain(),
        );
    }
}

// ==== CORE PLATFORMER SYSTEMS ====

// Constants for platformer physics
const MOVE_SPEED: f32 = 200.0;
const JUMP_FORCE: f32 = 400.0;
const GRAVITY: f32 = -800.0;
const MAX_FALL_SPEED: f32 = -500.0;
const PLAYER_SIZE: f32 = 30.0;
const PLATFORM_HEIGHT: f32 = 20.0;

// Handle player movement based on input
pub fn player_movement_system(
    mut query: Query<(&mut Player, &ActionState<PlayerActions>), With<Player>>,
) {
    for (mut player, action_state) in query.iter_mut() {
        // Horizontal movement
        let mut move_delta = 0.0;

        if action_state.pressed(&PlayerActions::MoveLeft) {
            move_delta -= 1.0;
        }
        if action_state.pressed(&PlayerActions::MoveRight) {
            move_delta += 1.0;
        }

        player.velocity.x = move_delta * MOVE_SPEED;

        // Jump (only when grounded)
        if action_state.just_pressed(&PlayerActions::Jump) && player.grounded {
            player.velocity.y = JUMP_FORCE;
            player.grounded = false;
        }
    }
}

// Update animation state based on player movement
pub fn update_animation_state_system(
    mut query: Query<(&Player, &mut PlayerAnimationState), With<Player>>,
) {
    for (player, mut anim_state) in query.iter_mut() {
        // Update movement state
        anim_state.is_moving = player.velocity.x.abs() > 10.0;
        
        // Update facing direction
        if player.velocity.x > 10.0 {
            anim_state.facing_left = false;
        } else if player.velocity.x < -10.0 {
            anim_state.facing_left = true;
        }
        
        // Update jumping state
        anim_state.is_jumping = !player.grounded;
    }
}

// Apply gravity to players
pub fn apply_gravity_system(
    mut query: Query<(&mut Player, &mut PlayerTransform)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (mut player, mut transform) in query.iter_mut() {
        // Apply gravity if not grounded
        if !player.grounded {
            player.velocity.y += GRAVITY * dt;
            player.velocity.y = player.velocity.y.max(MAX_FALL_SPEED);
        }

        // Apply velocity to position
        transform.translation.x += player.velocity.x * dt;
        transform.translation.y += player.velocity.y * dt;

        // Keep player in bounds (simple boundary)
        transform.translation.x = transform.translation.x.clamp(-400.0, 400.0);

        // Ground check (simple floor at y = -200)
        if transform.translation.y <= -200.0 {
            transform.translation.y = -200.0;
            player.velocity.y = 0.0;
            player.grounded = true;
        }
    }
}

// Detect if player is on ground or platform
pub fn ground_detection_system(
    mut players: Query<(&mut Player, &PlayerTransform), With<Player>>,
    platforms: Query<&Transform, (With<Platform>, Without<Player>)>,
) {
    for (mut player, player_transform) in players.iter_mut() {
        let player_bottom = player_transform.translation.y - PLAYER_SIZE / 2.0;
        let player_left = player_transform.translation.x - PLAYER_SIZE / 2.0;
        let player_right = player_transform.translation.x + PLAYER_SIZE / 2.0;

        // Check collision with platforms
        let mut on_platform = false;
        for platform_transform in platforms.iter() {
            let platform_top = platform_transform.translation.y + PLATFORM_HEIGHT / 2.0;
            let platform_bottom = platform_transform.translation.y - PLATFORM_HEIGHT / 2.0;
            let platform_left = platform_transform.translation.x - 100.0; // Platform width
            let platform_right = platform_transform.translation.x + 100.0;

            // Check if player is on top of platform
            if player_bottom <= platform_top
                && player_bottom >= platform_bottom
                && player_right >= platform_left
                && player_left <= platform_right
                && player.velocity.y <= 0.0
            {
                on_platform = true;
                break;
            }
        }

        // Update grounded state (also check floor)
        if on_platform || player_transform.translation.y <= -200.0 {
            if !player.grounded && player.velocity.y <= 0.0 {
                player.grounded = true;
                player.velocity.y = 0.0;
            }
        } else if player_transform.translation.y > -200.0 {
            player.grounded = false;
        }
    }
}

// ==== CUSTOM GAME SYSTEMS AREA - Add your game-specific systems here ====
// Example: Add new gameplay systems, AI, scoring, etc.
//
// pub fn my_custom_system(
//     // your queries and resources here
// ) {
//     // your custom logic
// }
//
// Remember to add your systems to the Plugin build() method above!
// ==== END CUSTOM GAME SYSTEMS AREA ====
