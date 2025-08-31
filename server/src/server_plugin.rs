use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
#[cfg(feature = "bevygap")]
use bevygap_server_plugin::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use shared::prelude::*;
use std::time::Duration;

#[derive(Default)]
pub struct BevygapSpaceshipsServerPlugin;

impl Plugin for BevygapSpaceshipsServerPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bevygap")]
        {
            app.add_plugins(BevygapServerPlugin::default());
            app.add_observer(handle_bevygap_ready);
        }

        app.add_systems(Startup, init);
        app.add_observer(handle_new_client);
        app.add_observer(handle_connected);
        
        // Add Leafwing input plugin for server
        app.add_plugins(input::leafwing::InputPlugin::<PlayerActions>::default());
        
        // Systems that handle player input and movement
        app.add_systems(
            FixedUpdate,
            (player_movement, shared_player_firing)
                .chain(),
        );
        
app.add_systems(Update, update_player_metrics);

        app.add_systems(
            FixedUpdate,
            handle_hit_event
                .run_if(on_event::<BulletHitEvent>)
                .after(process_collisions),
        );

        #[cfg(feature = "bevygap")]
        app.add_systems(
            Update,
            update_server_metadata.run_if(resource_added::<ArbitriumContext>),
        );
    }
}

#[cfg(feature = "bevygap")]
fn handle_bevygap_ready(_trigger: Trigger<BevygapReady>, mut commands: Commands) {
    info!("BevyGap reports ready - server can accept connections");
}

#[cfg(feature = "bevygap")]
fn update_server_metadata(
    mut metadata: ResMut<ServerMetadata>,
    context: Res<ArbitriumContext>,
    mut sender: ResMut<ServerResourceSender>,
) {
    metadata.fqdn = context.fqdn();
    metadata.location = context.location();
    metadata.build_info = format!(
        "Git: {} built at: {}",
        env!("VERGEN_GIT_DESCRIBE"),
        env!("VERGEN_BUILD_TIMESTAMP")
    );
    info!("Updating server metadata: {metadata:?}");
    
    // Replicate the ServerMetadata resource to all clients
    sender.replicate_resource::<ServerMetadata>(NetworkTarget::All);
}

/// When a new client connection is created, set up replication
pub(crate) fn handle_new_client(trigger: Trigger<OnAdd, LinkOf>, mut commands: Commands) {
    commands.entity(trigger.target()).insert((
        ReplicationSender::new(SERVER_REPLICATION_INTERVAL, SendUpdatesMode::SinceLastAck, false),
        Name::from("Client"),
    ));
}

/// When a client successfully connects, spawn their player entity
pub(crate) fn handle_connected(
    trigger: Trigger<OnAdd, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
    all_players: Query<Entity, With<Player>>,
) {
    let Ok(client_id) = query.get(trigger.target()) else {
        return;
    };
    let client_id = client_id.0;
    
    // Track the number of connected players for colors and positions
    let player_n = all_players.iter().count();
    
    info!("New connected client, client_id: {client_id:?}. Spawning player entity..");
    
    // Pick color and position for player
    let available_colors = [
        css::LIMEGREEN,
        css::PINK,
        css::YELLOW,
        css::AQUA,
        css::CRIMSON,
        css::GOLD,
        css::ORANGE_RED,
        css::SILVER,
        css::SALMON,
        css::YELLOW_GREEN,
        css::WHITE,
        css::RED,
    ];
    let col = available_colors[player_n % available_colors.len()];
    let angle: f32 = player_n as f32 * 5.0;
    let x = 200.0 * angle.cos();
    let y = 200.0 * angle.sin();

    // Spawn the player entity
    let player_ent = commands
        .spawn((
            Player::new(client_id, pick_player_name(client_id.to_bits())),
            Score(0),
            Name::new("Player"),
            ActionState::<PlayerActions>::default(),
            Position(Vec2::new(x, y)),
            Rotation::default(),
            LinearVelocity::default(),
            AngularVelocity::default(),
            // Replicate to all clients
            Replicate::to_clients(NetworkTarget::All),
            // Prediction for the owning client
            PredictionTarget::to_clients(NetworkTarget::Single(client_id)),
            // Interpolation for other clients
            InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(client_id)),
            // Control assignment
            ControlledBy {
                owner: trigger.target(),
                lifetime: Default::default(),
            },
            PhysicsBundle::player_ship(),
            Weapon::new((FIXED_TIMESTEP_HZ / 5.0) as u16),
            ColorComponent(col.into()),
        ))
        .id();

    info!("Created entity {player_ent:?} for client {client_id:?}");
}

/// Update player metrics (RTT, jitter) periodically
fn update_player_metrics(
    mut q: Query<&mut Player>,
) {
    // TODO: Update this when we have access to connection metrics
    // For now, just keep the existing values
    for mut player in q.iter_mut() {
        // Metrics will be updated when connection system is properly integrated
        player.rtt = Duration::from_millis(50);
        player.jitter = Duration::from_millis(5);
    }
}

fn init(mut commands: Commands) {
    #[cfg(feature = "gui")]
    {
        commands.spawn(
            TextBundle::from_section(
                "Server",
                TextStyle {
                    font_size: 30.0,
                    color: Color::WHITE,
                    ..default()
                },
            )
            .with_style(Style {
                align_self: AlignSelf::End,
                ..default()
            }),
        );
    }
    
    // Spawn server-authoritative balls
    const NUM_BALLS: usize = 6;
    for i in 0..NUM_BALLS {
        let radius = 10.0 + i as f32 * 4.0;
        let angle: f32 = i as f32 * (TAU / NUM_BALLS as f32);
        let pos = Vec2::new(125.0 * angle.cos(), 125.0 * angle.sin());
        commands.spawn(BallBundle::new(radius, pos, css::GOLD.into()));
    }
}

fn pick_player_name(client_id: u64) -> String {
    let index = (client_id % NAMES.len() as u64) as usize;
    NAMES[index].to_string()
}

const NAMES: [&str; 35] = [
    "Ellen Ripley",
    "Sarah Connor",
    "Neo",
    "Trinity",
    "Morpheus",
    "John Connor",
    "T-1000",
    "Rick Deckard",
    "Princess Leia",
    "Han Solo",
    "Spock",
    "James T. Kirk",
    "Hikaru Sulu",
    "Nyota Uhura",
    "Jean-Luc Picard",
    "Data",
    "Beverly Crusher",
    "Seven of Nine",
    "Doctor Who",
    "Rose Tyler",
    "Marty McFly",
    "Doc Brown",
    "Dana Scully",
    "Fox Mulder",
    "Riddick",
    "Barbarella",
    "HAL 9000",
    "Megatron",
    "Furiosa",
    "Lois Lane",
    "Clark Kent",
    "Tony Stark",
    "Natasha Romanoff",
    "Bruce Banner",
    "Mr. T",
];

/// Server handles scores when a bullet collides with a player
pub(crate) fn handle_hit_event(
    mut events: EventReader<BulletHitEvent>,
    mut player_q: Query<(&Player, &mut Score)>,
) {
    for ev in events.read() {
        // Find victim and update scores
        if let Some(victim_id) = ev.victim_client_id {
            // Decrease victim's score
            for (_player, mut score) in player_q.iter_mut() {
                if _player.client_id == victim_id {
                    score.0 -= 1;
                    break;
                }
            }
            
            // Increase shooter's score
            for (_player, mut score) in player_q.iter_mut() {
                if _player.client_id == ev.bullet_owner {
                    score.0 += 1;
                    break;
                }
            }
        }
    }
}

/// Read inputs and move players
pub(crate) fn player_movement(
    mut q: Query<(&ActionState<PlayerActions>, ApplyInputsQuery), With<Player>>,
    timeline: Single<&LocalTimeline, With<Server>>,
) {
    let tick = timeline.tick();
    for (action_state, mut aiq) in q.iter_mut() {
        apply_action_state_to_player_movement(action_state, 0, &mut aiq, tick);
    }
}
