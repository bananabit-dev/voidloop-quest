use bevy::prelude::*;
use bevygap_client_plugin::prelude::BevygapConnectExt;

// 🎮 Client-side lobby configuration
#[derive(Resource, Clone, Debug)]
pub struct LobbyConfig {
    pub domain: String,           // "voidloop.quest"
    pub matchmaker_url: String,   // "wss://voidloop.quest/matchmaker/ws"
    pub max_players: u32,         // 4 (changed from 16 for this implementation)
    pub lobby_modes: Vec<String>, // ["casual", "ranked", "custom"]
}

impl Default for LobbyConfig {
    fn default() -> Self {
        Self {
            domain: "voidloop.quest".to_string(),
            matchmaker_url: get_matchmaker_url(),
            max_players: 4,  // 🎯 Set to 4 players max as requested
            lobby_modes: vec![
                "casual".to_string(),
                "ranked".to_string(),
                "custom".to_string(),
            ],
        }
    }
}

// 🏠 Lobby UI component
#[derive(Component, Default)]
pub struct LobbyUI {
    pub current_players: u32,
    pub selected_mode: String,
    pub is_host: bool,
    pub is_searching: bool,
}

impl LobbyUI {
    pub fn new() -> Self {
        Self {
            current_players: 1, // Start with 1 (local player)
            selected_mode: "casual".to_string(),
            is_host: false,
            is_searching: false,
        }
    }
}

// 🎮 Game states
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    Lobby,
    InGame,
}

// 🌟 Lobby events
#[derive(Event)]
pub enum LobbyEvent {
    PlayerJoined(u32),
    PlayerLeft(u32),
    StartGame,
    SelectMode(String),
}

// 🎯 Lobby plugin
pub struct LobbyPlugin;

impl Plugin for LobbyPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_event::<LobbyEvent>()
            .insert_resource(LobbyConfig::default())
            .add_systems(OnEnter(AppState::Lobby), setup_lobby_ui)
            .add_systems(OnExit(AppState::Lobby), cleanup_lobby_ui)
            .add_systems(
                Update,
                (
                    handle_lobby_input,
                    update_lobby_ui,
                    handle_lobby_events,
                ).run_if(in_state(AppState::Lobby))
            );
    }
}

// 🏠 Initialize lobby system
fn setup_lobby_ui(mut commands: Commands) {
    info!("🏠 Setting up lobby UI");
    
    // Spawn lobby UI
    commands.spawn((
        LobbyUI::new(),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
    )).with_children(|parent| {
        // Title
        parent.spawn((
            Text::new("🎮 Voidloop Quest Lobby"),
            TextFont {
                font_size: 48.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::all(Val::Px(20.0)),
                ..default()
            },
        ));
        
        // Player count
        parent.spawn((
            Text::new("Players: 1/4"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            Node {
                margin: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            PlayerCountText,
        ));
        
        // Game mode selection
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                margin: UiRect::all(Val::Px(20.0)),
                ..default()
            },
        )).with_children(|mode_parent| {
            let modes = ["casual", "ranked", "custom"];
            for (i, mode) in modes.iter().enumerate() {
                mode_parent.spawn((
                    Button,
                    Node {
                        width: Val::Px(120.0),
                        height: Val::Px(40.0),
                        margin: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if i == 0 { Color::srgb(0.3, 0.6, 0.3) } else { Color::srgb(0.3, 0.3, 0.3) }),
                    ModeButton(mode.to_string()),
                )).with_children(|button_parent| {
                    button_parent.spawn((
                        Text::new(mode.to_uppercase()),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            }
        });
        
        // Connect button
        parent.spawn((
            Button,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(60.0),
                margin: UiRect::all(Val::Px(20.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.7, 0.2)),
            ConnectButton,
        )).with_children(|button_parent| {
            button_parent.spawn((
                Text::new("FIND MATCH"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                ConnectButtonText,
            ));
        });
        
        // Instructions
        parent.spawn((
            Text::new("Select a game mode and click 'FIND MATCH' to join a lobby with up to 4 players"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
            Node {
                margin: UiRect::all(Val::Px(20.0)),
                ..default()
            },
        ));
    });
}

// 🧹 Cleanup lobby UI when leaving lobby state
fn cleanup_lobby_ui(
    mut commands: Commands,
    lobby_query: Query<Entity, With<LobbyUI>>,
) {
    for entity in lobby_query.iter() {
        commands.entity(entity).despawn();
    }
}

// 🎮 Handle lobby input and button clicks
fn handle_lobby_input(
    mut commands: Commands,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, Option<&ModeButton>, Option<&ConnectButton>),
        (Changed<Interaction>, With<Button>)
    >,
    mut lobby_events: EventWriter<LobbyEvent>,
    mut lobby_ui_query: Query<&mut LobbyUI>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut color, mode_button, connect_button) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if let Some(mode_button) = mode_button {
                    // Mode selection
                    lobby_events.write(LobbyEvent::SelectMode(mode_button.0.clone()));
                    *color = BackgroundColor(Color::srgb(0.3, 0.6, 0.3));
                } else if connect_button.is_some() {
                    // Connect button pressed
                    info!("🔌 Starting matchmaking...");
                    
                    // Connect to matchmaker using BevyGap
                    commands.bevygap_connect_client();
                    
                    // Transition to game state
                    next_state.set(AppState::InGame);
                }
            },
            Interaction::Hovered => {
                if mode_button.is_some() {
                    *color = BackgroundColor(Color::srgb(0.4, 0.4, 0.4));
                } else if connect_button.is_some() {
                    *color = BackgroundColor(Color::srgb(0.3, 0.8, 0.3));
                }
            },
            Interaction::None => {
                if let Some(mode_button) = mode_button {
                    let lobby_ui = if let Ok(ui) = lobby_ui_query.single() {
                        ui
                    } else {
                        return;
                    };
                    if mode_button.0 == lobby_ui.selected_mode {
                        *color = BackgroundColor(Color::srgb(0.3, 0.6, 0.3));
                    } else {
                        *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
                    }
                } else if connect_button.is_some() {
                    *color = BackgroundColor(Color::srgb(0.2, 0.7, 0.2));
                }
            }
        }
    }
}

// 📊 Update lobby UI with current state
fn update_lobby_ui(
    lobby_config: Res<LobbyConfig>,
    lobby_ui_query: Query<&LobbyUI>,
    mut player_count_query: Query<&mut Text, With<PlayerCountText>>,
    mut connect_button_query: Query<(&mut Text, &mut BackgroundColor), (With<ConnectButtonText>, Without<PlayerCountText>)>,
) {
    if let Ok(lobby_ui) = lobby_ui_query.single() {
        // Update player count text
        if let Ok(mut text) = player_count_query.single_mut() {
            **text = format!("Players: {}/{}", lobby_ui.current_players, lobby_config.max_players);
        }
        
        // Update connect button based on state
        if let Ok((mut text, mut color)) = connect_button_query.single_mut() {
            if lobby_ui.is_searching {
                **text = "SEARCHING...".to_string();
                *color = BackgroundColor(Color::srgb(0.7, 0.7, 0.2));
            } else if lobby_ui.current_players >= lobby_config.max_players {
                **text = "LOBBY FULL".to_string();
                *color = BackgroundColor(Color::srgb(0.7, 0.2, 0.2));
            } else {
                **text = "FIND MATCH".to_string();
                *color = BackgroundColor(Color::srgb(0.2, 0.7, 0.2));
            }
        }
    }
}

// 🎯 Handle lobby events
fn handle_lobby_events(
    mut lobby_events: EventReader<LobbyEvent>,
    mut lobby_ui_query: Query<&mut LobbyUI>,
    mut mode_button_query: Query<(&mut BackgroundColor, &ModeButton), With<Button>>,
) {
    let mut lobby_ui = if let Ok(ui) = lobby_ui_query.single_mut() {
        ui
    } else {
        return;
    };
    
    for event in lobby_events.read() {
        match event {
            LobbyEvent::PlayerJoined(player_count) => {
                lobby_ui.current_players = *player_count;
                info!("🎮 Player joined! Current players: {}", lobby_ui.current_players);
            },
            LobbyEvent::PlayerLeft(player_count) => {
                lobby_ui.current_players = *player_count;
                info!("👋 Player left! Current players: {}", lobby_ui.current_players);
            },
            LobbyEvent::StartGame => {
                info!("🚀 Starting game with {} players!", lobby_ui.current_players);
                lobby_ui.is_searching = false;
            },
            LobbyEvent::SelectMode(mode) => {
                lobby_ui.selected_mode = mode.clone();
                info!("🎯 Selected game mode: {}", mode);
                
                // Update button colors
                for (mut color, mode_button) in mode_button_query.iter_mut() {
                    if mode_button.0 == *mode {
                        *color = BackgroundColor(Color::srgb(0.3, 0.6, 0.3));
                    } else {
                        *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
                    }
                }
            }
        }
    }
}

// Helper function to get matchmaker URL (similar to client_plugin.rs)
fn get_matchmaker_url() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        use web_sys;
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

// 🏷️ UI component markers
#[derive(Component)]
struct PlayerCountText;

#[derive(Component)]
struct ConnectButton;

#[derive(Component)]
struct ConnectButtonText;

#[derive(Component)]
struct ModeButton(String);