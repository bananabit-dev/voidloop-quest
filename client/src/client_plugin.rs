use crate::screens;


/// The game name sent to the matchmaker when requesting a server to play on
pub const GAME_NAME: &str = "bevygap-spaceships";
/// The game version sent to the matchmaker when requesting a server to play on
pub const GAME_VERSION: &str = "1";

pub struct BevygapSpaceshipsClientPlugin;

impl Plugin for BevygapSpaceshipsClientPlugin {
    fn build(&self, app: &mut App) {
        // will default to the Connect screen with a button to initiate
        app.add_plugins(screens::plugin);

        #[cfg(feature = "bevygap")]
        {
            let matchmaker_url = get_matchmaker_url();
            info!("Matchmaker url: {matchmaker_url}");
            app.insert_resource(BevygapClientConfig {
                matchmaker_url,
                game_name: GAME_NAME.to_string(),
                game_version: GAME_VERSION.to_string(),
                ..default()
            });

            app.add_plugins(BevygapClientPlugin);
            
            // Add Leafwing input plugin after BevyGap is set up
            app.add_plugins(input::leafwing::InputPlugin::<PlayerActions>::default());

            app.add_systems(
                Update,
                on_bevygap_state_change.run_if(state_changed::<BevygapClientState>),
            );
        }
        
        #[cfg(not(feature = "bevygap"))]
        {
            // Add Leafwing input plugin for non-bevygap builds
            app.add_plugins(input::leafwing::InputPlugin::<PlayerActions>::default());
        }

        app.add_observer(connect_client_observer);

        // all actions related-system that can be rolled back should be in FixedUpdate schedule
        app.add_systems(
            FixedUpdate,
            (
                player_movement,
                shared_player_firing.run_if(not(is_in_rollback)),
            )
            .run_if(resource_exists::<ClientConnection>),
        );
        app.add_systems(Update, (add_ball_physics, add_bullet_physics, handle_new_player));

        app.add_systems(
            Update,
            render_server_metadata
                .run_if(resource_exists::<ServerMetadata>)
                .run_if(resource_changed::<ServerMetadata>),
        );

        // show the client id when connection is established
        app.add_observer(handle_connection);
    }
}

use bevy::prelude::*;
#[cfg(feature = "bevygap")]
use bevygap_client_plugin::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::input::InputBuffer;
use shared::prelude::*;

/// This is the path to the websocket endpoint on `bevygap_matchmaker_httpd``
///
/// * Checks for window.MATCHMAKER_URL global variable (set in index.html)
///
/// otherwise, defaults to transforming the window.location:
///
/// * Changes http://www.example.com/whatever  -> ws://www.example.com/matchmaker/ws
/// * Changes https://www.example.com/whatever -> wss://www.example.com/matchmaker/ws
#[cfg(target_family = "wasm")]
fn get_matchmaker_url() -> String {
    const MATCHMAKER_PATH: &str = "/matchmaker/ws";
    let window = web_sys::window().expect("expected window");
    if let Some(obj) = window.get("MATCHMAKER_URL") {
        info!("Using matchmaker url from window.MATCHMAKER_URL");
        obj.as_string().expect("MATCHMAKER_URL should be a string")
    } else {
        info!("Creating matchmaker url from window.location");
        let location = window.location();
        let host = location.host().expect("Expected host");
        let proto = if location.protocol().expect("Expected protocol") == "https:" {
            "wss:"
        } else {
            "ws:"
        };
        format!("{proto}//{host}{MATCHMAKER_PATH}")
    }
}

/// This is the path to the websocket endpoint on `bevygap_matchmaker_httpd``
///
/// * Reads COMPILE_TIME_MATCHMAKER_URL environment variable during compilation
/// otherwise:
/// * Reads the MATCHMAKER_URL environment variable at runtime
/// otherwise:
/// * Defaults to a localhost dev url.
#[cfg(not(target_family = "wasm"))]
fn get_matchmaker_url() -> String {
    const MATCHMAKER_PATH: &str = "/matchmaker/ws";
    // use compile-time env variable, this overwrites everything if set.
    match option_env!("COMPILE_TIME_MATCHMAKER_URL") {
        Some(url) => {
            info!("Using matchmaker url from COMPILE_TIME_MATCHMAKER_URL env");
            url.to_string()
        }
        None => {
            if let Ok(url) = std::env::var("MATCHMAKER_URL") {
                info!("Using matchmaker url from MATCHMAKER_URL env");
                url
            } else {
                warn!("Using default localhost dev url for matchmaker");
                format!("ws://127.0.0.1:3000{MATCHMAKER_PATH}")
            }
        }
    }
}

/// Starts the lightyear connection process (via bevygap), when ConnectToServerRequest event is triggered.
#[cfg(feature = "bevygap")]
pub(crate) fn connect_client_observer(
    _trigger: Trigger<crate::screens::ConnectToServerRequest>,
    mut commands: Commands,
    state: Res<State<BevygapClientState>>,
) {
    info!("Connecting...");
    match state.get() {
        BevygapClientState::Dormant | BevygapClientState::Error(_, _) => {
            commands.bevygap_connect_client();
        }
        _ => {
            warn!("Already trying to connect");
        }
    }
}

/// Starts the lightyear connection process, when ConnectToServerRequest event is triggered.
#[cfg(not(feature = "bevygap"))]
pub(crate) fn connect_client_observer(
    _trigger: Trigger<crate::screens::ConnectToServerRequest>,
    mut commands: Commands,
    q_client: Query<Entity, With<Client>>,
) {
    info!("Connecting...");
    if let Ok(entity) = q_client.get_single() {
        commands.trigger_targets(Connect, entity);
    } else {
        warn!("No Client entity found to connect");
    }
}

#[cfg(feature = "bevygap")]
fn on_bevygap_state_change(state: Res<State<BevygapClientState>>) {
    info!("Bevygap client state = {state:?}");
}

fn render_server_metadata(mut commands: Commands, metadata: Res<ServerMetadata>) {
    if metadata.fqdn.is_empty() {
        return;
    }
    // logs will include the build info: timestamp and git sha of server you've connected to.
    // but this isn't shown in the UI.
    info!("Got server metadata: {:?}", metadata);
    commands
        .spawn((
            Text::default(),
            TextFont::from_font_size(16.0),
            TextColor(bevy::color::palettes::css::WHITE.into()),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn(TextSpan::new(format!(
                "Server {} @ {}",
                metadata.fqdn, metadata.location
            )));
        });
}

/// Listen for connection state changes and spawn a text entity to display the client id
fn handle_connection(_trigger: Trigger<OnAdd, Connected>, mut commands: Commands, local_id: Single<&LocalId>) {
    commands
        .spawn((
            Text::default(),
            TextFont::from_font_size(12.0),
            TextColor(Color::WHITE),
            Node { position_type: PositionType::Absolute, top: Val::Px(25.0), left: Val::Px(5.0), ..default() },
        ))
        .with_children(|p| {
            p.spawn(TextSpan::new(format!("Client {}", local_id.0)));
        });
}

/// Blueprint pattern: when the ball gets replicated from the server, add all the components
/// that we need that are not replicated.
/// (for example physical properties that are constant, so they don't need to be networked)
///
/// We only add the physical properties on the ball that is displayed on screen (i.e the Predicted ball)
/// We want the ball to be rigid so that when players collide with it, they bounce off.
fn add_ball_physics(
    mut commands: Commands,
    mut ball_query: Query<(Entity, &BallMarker), Added<Predicted>>,
) {
    for (entity, ball) in ball_query.iter_mut() {
        info!("Adding physics to a replicated ball {entity:?}");
        commands.entity(entity).insert(ball.physics_bundle());
    }
}

/// Simliar blueprint scenario as balls, except sometimes clients prespawn bullets ahead of server
/// replication, which means they will already have the physics components.
/// So, we filter the query using `Without<Collider>`.
fn add_bullet_physics(
    mut commands: Commands,
    mut bullet_query: Query<Entity, (With<BulletMarker>, Added<Predicted>, Without<Collider>)>,
) {
    for entity in bullet_query.iter_mut() {
        info!("Adding physics to a replicated bullet:  {entity:?}");
        commands.entity(entity).insert(PhysicsBundle::bullet());
    }
}

/// Decorate newly connecting players with physics components
/// ..and if it's our own player, set up input stuff
#[allow(clippy::type_complexity)]
fn handle_new_player(
    mut commands: Commands,
    mut player_query: Query<(Entity, Has<Controlled>), (Added<Predicted>, With<Player>)>,
) {
    for (entity, is_controlled) in player_query.iter_mut() {
        // is this our own entity?
        if is_controlled {
            info!("Own player replicated to us, adding inputmap {entity:?}");
            commands.entity(entity).insert(InputMap::new([
                (PlayerActions::Up, KeyCode::ArrowUp),
                (PlayerActions::Down, KeyCode::ArrowDown),
                (PlayerActions::Left, KeyCode::ArrowLeft),
                (PlayerActions::Right, KeyCode::ArrowRight),
                (PlayerActions::Up, KeyCode::KeyW),
                (PlayerActions::Down, KeyCode::KeyS),
                (PlayerActions::Left, KeyCode::KeyA),
                (PlayerActions::Right, KeyCode::KeyD),
                (PlayerActions::Fire, KeyCode::Space),
            ]));
        } else {
            info!("Remote player replicated to us: {entity:?}");
        }
        commands.entity(entity).insert(PhysicsBundle::player_ship());
    }
}

// Generate an explosion effect for bullet collisions
fn handle_hit_event(
    time: Res<Time>,
    mut events: EventReader<BulletHitEvent>,
    mut commands: Commands,
) {
    for ev in events.read() {
        #[cfg(feature = "gui")]
        {
            use shared::renderer::Explosion;
            commands.spawn((
                Transform::from_xyz(ev.position.x, ev.position.y, 0.0),
                Visibility::default(),
                Explosion::new(time.elapsed(), ev.bullet_color),
            ));
        }
    }
}

// only apply movements to predicted entities
fn player_movement(
    mut q: Query<(
        &ActionState<PlayerActions>,
        &InputBuffer<ActionState<PlayerActions>>,
        ApplyInputsQuery,
    ), (With<Player>, With<Predicted>)>,
    timeline: Single<&LocalTimeline>,
) {
    // derive tick from local timeline (works across rollback)
    let tick = timeline.tick();

    for (action_state, input_buffer, mut aiq) in q.iter_mut() {
        if input_buffer.get(tick).is_some() {
            apply_action_state_to_player_movement(action_state, 0, &mut aiq, tick);
            continue;
        }
        if let Some((prev_tick, prev_input)) = input_buffer.get_last_with_tick() {
            let staleness = (tick - prev_tick).max(0) as u16;
            const MAX_STALE_TICKS: u16 = 6;
            if staleness > MAX_STALE_TICKS {
                apply_action_state_to_player_movement(&ActionState::default(), staleness, &mut aiq, tick);
            } else {
                let mut snapshot = ActionState::<PlayerActions>::default();
                snapshot.clone_from(prev_input);
                apply_action_state_to_player_movement(&snapshot, staleness, &mut aiq, tick);
            }
        } else {
            apply_action_state_to_player_movement(action_state, 0, &mut aiq, tick);
        }
    }
}
