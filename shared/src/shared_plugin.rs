use crate::prelude::*;
use bevy::{ecs::query::QueryData, prelude::*};
use bevy::prelude::Single;
use crate::protocol_plugin::{
    PlayerActions, BulletBundle, PhysicsBundle, ColorComponent, BulletHitEvent, Player,
    BulletMarker, Lifetime, SHIP_LENGTH, BULLET_SIZE, Weapon, ProtocolPlugin,
};
use lightyear::prelude::*;

pub struct BevygapSpaceshipsSharedPlugin;

impl Plugin for BevygapSpaceshipsSharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProtocolPlugin);

        #[cfg(feature = "gui")]
        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.add_plugins(BLEMRendererPlugin);
        }
        // bundles
        app.add_systems(Startup, init);
        // Physics
        //
        // we use Position and Rotation as primary source of truth, so no need to sync changes
        // from Transform->Pos, just Pos->Transform.
        app.insert_resource(avian2d::sync::SyncConfig {
            transform_to_position: false,
            transform_to_collider_scale: true,
            position_to_transform: true,
        });
        // We change SyncPlugin to PostUpdate, because we want the visually interpreted values
        // synced to transform every time, not just when Fixed schedule runs.
        app.add_plugins(
            PhysicsPlugins::new(FixedUpdate)
                .build()
                .disable::<avian2d::sync::SyncPlugin>(),
        )
        .add_plugins(SyncPlugin::new(PostUpdate));

        app.init_resource::<Time<Physics>>();
        app.insert_resource(SubstepCount(1));
        app.insert_resource(Gravity(Vec2::ZERO));

        app.add_systems(FixedUpdate, process_collisions);
        app.add_systems(FixedUpdate, lifetime_despawner);

        app.add_event::<BulletHitEvent>();
        // registry types for reflection
        app.register_type::<Player>();
    }
}

// Generate pseudo-random color from id
pub fn color_from_id(client_id: PeerId) -> Color {
    let h = (((client_id.to_bits().wrapping_mul(30)) % 360) as f32) / 360.0;
    let s = 1.0;
    let l = 0.5;
    Color::hsl(h, s, l)
}

fn init(mut commands: Commands) {
    commands.spawn(WallBundle::new(
        Vec2::new(-WALL_SIZE, -WALL_SIZE),
        Vec2::new(-WALL_SIZE, WALL_SIZE),
        Color::WHITE,
    ));
    commands.spawn(WallBundle::new(
        Vec2::new(-WALL_SIZE, WALL_SIZE),
        Vec2::new(WALL_SIZE, WALL_SIZE),
        Color::WHITE,
    ));
    commands.spawn(WallBundle::new(
        Vec2::new(WALL_SIZE, WALL_SIZE),
        Vec2::new(WALL_SIZE, -WALL_SIZE),
        Color::WHITE,
    ));
    commands.spawn(WallBundle::new(
        Vec2::new(WALL_SIZE, -WALL_SIZE),
        Vec2::new(-WALL_SIZE, -WALL_SIZE),
        Color::WHITE,
    ));
}

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
pub struct ApplyInputsQuery {
    pub ex_force: &'static mut ExternalForce,
    pub ang_vel: &'static mut AngularVelocity,
    pub rot: &'static Rotation,
    pub player: &'static Player,
}

/// applies forces based on action state inputs
pub fn apply_action_state_to_player_movement(
    action: &ActionState<PlayerActions>,
    _staleness: u16,
    aiq: &mut ApplyInputsQueryItem,
    _tick: Tick,
) {
    let ex_force = &mut aiq.ex_force;
    let rot = &aiq.rot;
    let ang_vel = &mut aiq.ang_vel;

    const THRUSTER_POWER: f32 = 32000.;
    const ROTATIONAL_SPEED: f32 = 4.0;

    if action.pressed(&PlayerActions::Up) {
        ex_force
            .apply_force(*rot * (Vec2::Y * THRUSTER_POWER))
            .with_persistence(false);
    }
    let desired_ang_vel = if action.pressed(&PlayerActions::Left) {
        ROTATIONAL_SPEED
    } else if action.pressed(&PlayerActions::Right) {
        -ROTATIONAL_SPEED
    } else {
        0.0
    };
    if ang_vel.0 != desired_ang_vel {
        ang_vel.0 = desired_ang_vel;
    }
}

/// Spawn bullets on server or for locally-controlled player
pub fn shared_player_firing(
    mut q: Query<
        (
            &Position,
            &Rotation,
            &LinearVelocity,
            &ColorComponent,
            &ActionState<PlayerActions>,
            &mut Weapon,
            Has<Controlled>,
            &Player,
        ),
        Or<(With<Predicted>, With<Replicate>)>,
    >,
    mut commands: Commands,
    timeline: Single<(&LocalTimeline, Has<Server>), Without<Client>>,
) {
    if q.is_empty() {
        return;
    }

    let (timeline, is_server) = timeline.into_inner();
    let current_tick = timeline.tick();
    for (
        player_position,
        player_rotation,
        player_velocity,
        color,
        action,
        mut weapon,
        is_local,
        player,
    ) in q.iter_mut()
    {
        if !is_server && !is_local {
            continue;
        }
        if !action.pressed(&PlayerActions::Fire) {
            continue;
        }
        let wrapped_diff = weapon.last_fire_tick - current_tick;
        if wrapped_diff.abs() <= weapon.cooldown as i16 {
            if weapon.last_fire_tick == current_tick {
                info!("Can't fire, fired this tick already! {current_tick:?}");
            }
            continue;
        }
        let prev_last_fire_tick = weapon.last_fire_tick;
        weapon.last_fire_tick = current_tick;

        let bullet_spawn_offset = Vec2::Y * (2.0 + (SHIP_LENGTH + BULLET_SIZE) / 2.0);

        let bullet_origin = player_position.0 + player_rotation * bullet_spawn_offset;
        let bullet_linvel = player_rotation * (Vec2::Y * weapon.bullet_speed) + player_velocity.0;

        let prespawned = PreSpawned::default_with_salt(player.client_id.to_bits());

        let bullet_entity = commands
            .spawn((
                BulletBundle::new(
                    player.client_id,
                    bullet_origin,
                    bullet_linvel,
                    (color.0.to_linear() * 5.0).into(),
                    current_tick,
                ),
                PhysicsBundle::bullet(),
                prespawned,
            ))
            .id();
        info!(
            "spawned bullet for ActionState, bullet={bullet_entity:?} ({}, {}). prev last_fire tick: {prev_last_fire_tick:?}",
            weapon.last_fire_tick.0, player.client_id
        );

        if is_server {
            let replicate = Replicate::to_clients(NetworkTarget::All);
            commands.entity(bullet_entity).insert(replicate);
        }
    }
}

pub fn lifetime_despawner(
    q: Query<(Entity, &Lifetime)>,
    mut commands: Commands,
    timeline: Single<&LocalTimeline>,
) {
    for (e, ttl) in q.iter() {
        if (timeline.tick() - ttl.origin_tick) > ttl.lifetime {
            commands.entity(e).prediction_despawn();
        }
    }
}

// Wall
#[derive(Bundle)]
pub struct WallBundle {
    color: ColorComponent,
    physics: PhysicsBundle,
    wall: Wall,
    name: Name,
}

#[derive(Component)]
pub struct Wall {
    pub start: Vec2,
    pub end: Vec2,
}

impl WallBundle {
    pub fn new(start: Vec2, end: Vec2, color: Color) -> Self {
        Self {
            color: ColorComponent(color),
            physics: PhysicsBundle {
                collider: Collider::segment(start, end),
                collider_density: ColliderDensity(1.0),
                rigid_body: RigidBody::Static,
                external_force: ExternalForce::default(),
            },
            wall: Wall { start, end },
            name: Name::new("Wall"),
        }
    }
}

pub fn process_collisions(
    collisions: Collisions,
    bullet_q: Query<(&BulletMarker, &ColorComponent, &Position)>,
    player_q: Query<&Player>,
    mut commands: Commands,
    timeline: Single<(&LocalTimeline, Has<Server>), Without<Client>>,
    mut hit_ev_writer: EventWriter<BulletHitEvent>,
) {
    let (_timeline, _is_server) = timeline.into_inner();
    for contacts in collisions.iter() {
        if let Ok((bullet, col, bullet_pos)) = bullet_q.get(contacts.collider1) {
            if player_q.get(contacts.collider2).is_ok() {
                // own bullet colliding with owner: ignore
                continue;
            }
            commands.entity(contacts.collider1).prediction_despawn();
            let victim_client_id = player_q
                .get(contacts.collider2)
                .ok()
                .map(|victim_player| victim_player.client_id);

            let ev = BulletHitEvent {
                bullet_owner: bullet.owner,
                victim_client_id,
                position: bullet_pos.0,
                bullet_color: col.0,
            };
            hit_ev_writer.write(ev);
        }
        if let Ok((bullet, col, bullet_pos)) = bullet_q.get(contacts.collider2) {
            if player_q.get(contacts.collider1).is_ok() {
                continue;
            }
            commands.entity(contacts.collider2).prediction_despawn();
            let victim_client_id = player_q
                .get(contacts.collider1)
                .ok()
                .map(|victim_player| victim_player.client_id);

            let ev = BulletHitEvent {
                bullet_owner: bullet.owner,
                victim_client_id,
                position: bullet_pos.0,
                bullet_color: col.0,
            };
            hit_ev_writer.write(ev);
        }
    }
}
