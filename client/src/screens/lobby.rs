use bevy::prelude::*;
use rand::Rng;

#[cfg(feature = "bevygap")]
use bevygap_client_plugin::prelude::BevygapConnectExt;

#[cfg(feature = "bevygap")]
use edgegap_async::{
    apis::{configuration::Configuration, lobbies_api},
    models::{LobbyCreatePayload, LobbyDeployPayload, LobbyReadResponse},
};

#[cfg(all(feature = "bevygap", not(target_arch = "wasm32")))]
use tokio::runtime::Runtime;

#[cfg(all(feature = "bevygap", target_arch = "wasm32"))]
use wasm_bindgen_futures::spawn_local;

use shared::RoomInfo;

#[derive(Resource, Default)]
pub struct ClientRoomRegistry {
    pub rooms: Vec<RoomInfo>,
}

#[derive(Resource, Default)]
pub struct ConnectionState {
    // Reserved for future connection state tracking
}

// Resource for tracking Edgegap lobby state
#[cfg(feature = "bevygap")]
#[derive(Resource, Default)]
pub struct EdgegapLobbyState {
    pub lobby_name: Option<String>,
    pub lobby_response: Option<LobbyReadResponse>,
    pub is_deploying: bool,
    pub deployment_error: Option<String>,
}

#[cfg(not(feature = "bevygap"))]
#[derive(Resource, Default)]
pub struct EdgegapLobbyState;

#[derive(Resource, Clone, Debug)]
pub struct LobbyConfig {
    pub domain: String,           // "voidloop.quest"
    pub matchmaker_url: String,   // "wss://voidloop.quest/matchmaker/ws"
    pub max_players: u32,         // 4 (changed from 16 for this implementation)
    pub lobby_modes: Vec<String>, // ["casual", "ranked", "custom"]
    #[cfg(feature = "bevygap")]
    pub edgegap_api_url: String, // Edgegap API base URL
    #[cfg(feature = "bevygap")]
    pub edgegap_token: Option<String>, // Edgegap API token
}

impl Default for LobbyConfig {
    fn default() -> Self {
        Self {
            domain: "voidloop.quest".to_string(),
            matchmaker_url: get_matchmaker_url(),
            max_players: 4, // 🎯 Set to 4 players max as requested
            lobby_modes: vec![
                "casual".to_string(),
                "ranked".to_string(),
                "custom".to_string(),
            ],
            #[cfg(feature = "bevygap")]
            edgegap_api_url: std::env::var("EDGEGAP_BASE_URL")
                .unwrap_or_else(|_| "https://api.edgegap.com".to_string()),
            #[cfg(feature = "bevygap")]
            edgegap_token: std::env::var("EDGEGAP_TOKEN").ok(),
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
    pub room_id: String,
    pub lobby_mode: LobbyMode,
    pub available_rooms: Vec<RoomInfo>,
    pub player_name: String,
}

impl LobbyUI {
    pub fn new() -> Self {
        Self {
            current_players: 1, // Start with 1 (local player)
            selected_mode: "casual".to_string(),
            is_host: false,
            is_searching: false,
            room_id: String::new(),
            lobby_mode: LobbyMode::Main,
            available_rooms: Vec::new(),
            player_name: format!("Player{}", rand::random::<u32>() % 1000),
        }
    }
}

// Different lobby screens/modes
#[derive(Default, Clone, PartialEq)]
pub enum LobbyMode {
    #[default]
    Main,
    CreateRoom,
    JoinRoom,
    InRoom,
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
    StartLocalGame,
    SelectMode(String),
    CreateRoom,
    ConfirmCreateRoom,
    JoinRoom,
    EnterRoomId(String),
    LeaveRoom,
    // New events for real matchmaking
    StartMatchmaking,
    RequestRoomList,
    RoomListReceived(Vec<RoomInfo>),
    LobbyCreated(String), // lobby name
    #[cfg(feature = "bevygap")]
    LobbyDeployed(LobbyReadResponse),
    LobbyDeploymentFailed(String),
    ConnectedToServer,
}

// 🎯 Lobby plugin
pub struct LobbyPlugin;

impl Plugin for LobbyPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_event::<LobbyEvent>()
            .insert_resource(LobbyConfig::default())
            .insert_resource(ConnectionState::default())
            .insert_resource(EdgegapLobbyState::default())
            .insert_resource(ClientRoomRegistry::default())
            .add_systems(OnEnter(AppState::Lobby), setup_lobby_ui)
            .add_systems(OnExit(AppState::Lobby), cleanup_lobby_ui)
            .add_systems(
                Update,
                (
                    handle_lobby_input,
                    update_lobby_display,
                    update_simple_ui,
                    handle_lobby_events,
                    handle_connection_events,
                    #[cfg(feature = "bevygap")]
                    handle_matchmaking_events,
                )
                    .run_if(in_state(AppState::Lobby)),
            );
    }
}

// 🏠 Initialize lobby system
fn setup_lobby_ui(mut commands: Commands, _asset_server: Res<AssetServer>) {
    info!("🏠 Setting up lobby UI - DEBUG");

    // Spawn main lobby UI container
    commands.spawn((
        LobbyUI::new(),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Percent(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.2)), // Dark blue background
        LobbyContainer,
    ));
}

// Update lobby UI based on current mode
fn update_lobby_display(
    mut commands: Commands,
    lobby_ui_query: Query<(&LobbyUI, Entity), (With<LobbyContainer>, Changed<LobbyUI>)>,
    existing_ui: Query<Entity, (With<LobbyUIElements>, Without<LobbyContainer>)>,
) {
    if let Ok((lobby_ui, container_entity)) = lobby_ui_query.single() {
        // Clear existing UI elements safely
        for entity in existing_ui.iter() {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
        }

        // Rebuild UI based on current mode
        match lobby_ui.lobby_mode {
            LobbyMode::Main => {
                spawn_main_lobby_ui(&mut commands, container_entity, lobby_ui);
            }
            LobbyMode::CreateRoom => {
                spawn_create_room_ui(&mut commands, container_entity, lobby_ui);
            }
            LobbyMode::JoinRoom => {
                spawn_join_room_ui(&mut commands, container_entity, lobby_ui);
            }
            LobbyMode::InRoom => {
                spawn_in_room_ui(&mut commands, container_entity, lobby_ui);
            }
        }
    }
}

fn spawn_main_lobby_ui(commands: &mut Commands, container_entity: Entity, _lobby_ui: &LobbyUI) {
    let title_entity = commands
        .spawn((
            Text::new("🎮 Voidloop Quest"),
            TextFont {
                font_size: 32.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Node {
                margin: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            LobbyUIElements,
        ))
        .id();

    // Mode buttons container
    let mode_container = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                margin: UiRect::all(Val::Px(15.0)),
                ..default()
            },
            LobbyUIElements,
        ))
        .id();

    // Add mode buttons as children
    let modes = ["casual", "ranked", "custom"];
    for (i, mode) in modes.iter().enumerate() {
        let button_entity = commands
            .spawn((
                Button,
                Node {
                    width: Val::Px(100.0),
                    height: Val::Px(40.0),
                    margin: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(if i == 0 {
                    Color::srgb(0.4, 0.7, 0.4)
                } else {
                    Color::srgb(0.3, 0.3, 0.3)
                }),
                ModeButton(mode.to_string()),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new(mode.to_uppercase()),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                ));
            })
            .id();
        commands.entity(mode_container).add_child(button_entity);
    }

    // Room management buttons container
    let button_container = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            LobbyUIElements,
        ))
        .id();

    // Quick match button (NEW)
    let quick_match_btn = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(180.0),
                height: Val::Px(50.0),
                margin: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.6, 0.2, 0.6)),
            QuickMatchButton,
            LobbyUIElements,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new("🎯 QUICK MATCH"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));
        })
        .id();

    // Create room button
    let create_btn = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(180.0),
                height: Val::Px(50.0),
                margin: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
            CreateRoomButton,
            LobbyUIElements,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new("CREATE ROOM"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));
        })
        .id();

    // Join room button
    let join_btn = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(180.0),
                height: Val::Px(50.0),
                margin: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.4, 0.6)),
            JoinRoomButton,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new("JOIN ROOM"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));
        })
        .id();

    // Local play button
    let local_btn = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(180.0),
                height: Val::Px(50.0),
                margin: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.6, 0.4, 0.2)),
            LocalPlayButton,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new("LOCAL PLAY"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));
        })
        .id();

    // Add all buttons to container
    commands.entity(button_container).add_child(quick_match_btn);
    commands.entity(button_container).add_child(create_btn);
    commands.entity(button_container).add_child(join_btn);
    commands.entity(button_container).add_child(local_btn);

    // Add all elements to main container
    commands.entity(container_entity).add_child(title_entity);
    commands.entity(container_entity).add_child(mode_container);
    commands
        .entity(container_entity)
        .add_child(button_container);
}

fn spawn_create_room_ui(commands: &mut Commands, container_entity: Entity, lobby_ui: &LobbyUI) {
    let title = commands
        .spawn((
            Text::new("Create Room"),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Node {
                margin: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            LobbyUIElements,
        ))
        .id();

    let room_info = commands
        .spawn((
            Text::new(format!(
                "Room ID: {}",
                if lobby_ui.room_id.is_empty() {
                    "Auto-generated"
                } else {
                    &lobby_ui.room_id
                }
            )),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            Node {
                margin: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            LobbyUIElements,
        ))
        .id();

    let create_btn = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(150.0),
                height: Val::Px(50.0),
                margin: UiRect::all(Val::Px(15.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
            ConfirmCreateButton,
            LobbyUIElements,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new("CREATE"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));
        })
        .id();

    let back_btn = spawn_back_button_simple(commands);

    commands.entity(container_entity).add_child(title);
    commands.entity(container_entity).add_child(room_info);
    commands.entity(container_entity).add_child(create_btn);
    commands.entity(container_entity).add_child(back_btn);
}

fn spawn_join_room_ui(commands: &mut Commands, container_entity: Entity, lobby_ui: &LobbyUI) {
    let title = commands
        .spawn((
            Text::new("Join Room"),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Node {
                margin: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            LobbyUIElements,
        ))
        .id();

    let room_input = commands
        .spawn((
            Text::new(format!("Enter Room ID: {}", lobby_ui.room_id)),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            Node {
                margin: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            LobbyUIElements,
        ))
        .id();

    // Available rooms display
    let rooms_container = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            LobbyUIElements,
        ))
        .id();

    // Show available rooms or loading message
    if lobby_ui.available_rooms.is_empty() {
        let loading_text = commands
            .spawn((
                Text::new("Loading rooms..."),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                Node {
                    margin: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                LobbyUIElements,
            ))
            .id();
        commands.entity(rooms_container).add_child(loading_text);
    } else {
        for room in &lobby_ui.available_rooms {
            let room_text = format!(
                "{} ({}/{}) - {}",
                room.room_id, room.current_players, room.max_players, room.game_mode
            );
            let room_btn = commands
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(35.0),
                        margin: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                    RoomIdButton(room.room_id.clone()),
                    LobbyUIElements,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(room_text),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    ));
                })
                .id();
            commands.entity(rooms_container).add_child(room_btn);
        }
    }

    let join_btn = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(150.0),
                height: Val::Px(50.0),
                margin: UiRect::all(Val::Px(15.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.4, 0.6)),
            ConfirmJoinButton,
            LobbyUIElements,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new("JOIN"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));
        })
        .id();

    let back_btn = spawn_back_button_simple(commands);

    commands.entity(container_entity).add_child(title);
    commands.entity(container_entity).add_child(room_input);
    commands.entity(container_entity).add_child(rooms_container);
    commands.entity(container_entity).add_child(join_btn);
    commands.entity(container_entity).add_child(back_btn);
}

fn spawn_in_room_ui(commands: &mut Commands, container_entity: Entity, lobby_ui: &LobbyUI) {
    let title = commands
        .spawn((
            Text::new(format!("Room: {}", lobby_ui.room_id)),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Node {
                margin: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            LobbyUIElements,
        ))
        .id();

    let player_count = commands
        .spawn((
            Text::new(format!("Players: {}/4", lobby_ui.current_players)),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            Node {
                margin: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            PlayerCountText,
            LobbyUIElements,
        ))
        .id();

    commands.entity(container_entity).add_child(title);
    commands.entity(container_entity).add_child(player_count);

    // Host indicator
    if lobby_ui.is_host {
        let host_indicator = commands
            .spawn((
                Text::new("👑 You are the host"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.2)),
                Node {
                    margin: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                LobbyUIElements,
            ))
            .id();
        commands.entity(container_entity).add_child(host_indicator);
    }

    // Status
    let status_text = if lobby_ui.is_searching {
        "🔍 Creating game server..."
    } else if lobby_ui.current_players >= 1 {
        "✅ Ready to play!"
    } else {
        "⏳ Waiting for players..."
    };

    let status = commands
        .spawn((
            Text::new(status_text),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.9, 0.7)),
            Node {
                margin: UiRect::all(Val::Px(15.0)),
                ..default()
            },
            LobbyUIElements,
        ))
        .id();
    commands.entity(container_entity).add_child(status);

    // Action buttons container
    let button_container = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                margin: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            LobbyUIElements,
        ))
        .id();

    // Start game button
    if lobby_ui.is_host || lobby_ui.current_players >= 1 {
        let start_btn = commands
            .spawn((
                Button,
                Node {
                    width: Val::Px(120.0),
                    height: Val::Px(50.0),
                    margin: UiRect::all(Val::Px(10.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
                StartGameButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("START GAME"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                ));
            })
            .id();
        commands.entity(button_container).add_child(start_btn);
    }

    // Leave room button
    let leave_btn = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(120.0),
                height: Val::Px(50.0),
                margin: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.6, 0.2, 0.2)),
            LeaveRoomButton,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new("LEAVE ROOM"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));
        })
        .id();
    commands.entity(button_container).add_child(leave_btn);

    commands
        .entity(container_entity)
        .add_child(button_container);
}

fn spawn_back_button_simple(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Button,
            Node {
                width: Val::Px(100.0),
                height: Val::Px(40.0),
                margin: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.4, 0.4, 0.4)),
            BackButton,
            LobbyUIElements,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new("BACK"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));
        })
        .id()
}

// 🧹 Cleanup lobby UI when leaving lobby state
fn cleanup_lobby_ui(mut commands: Commands, lobby_query: Query<Entity, With<LobbyContainer>>) {
    for entity in lobby_query.iter() {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.despawn();
        }
    }
}

// 🎮 Handle lobby input and button clicks
fn handle_lobby_input(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, Entity),
        (Changed<Interaction>, With<Button>),
    >,
    button_types: Query<(
        Option<&ModeButton>,
        Option<&QuickMatchButton>,
        Option<&CreateRoomButton>,
        Option<&JoinRoomButton>,
        Option<&LocalPlayButton>,
        Option<&ConfirmCreateButton>,
        Option<&ConfirmJoinButton>,
        Option<&RoomIdButton>,
        Option<&StartGameButton>,
        Option<&LeaveRoomButton>,
        Option<&BackButton>,
    )>,
    mut lobby_events: EventWriter<LobbyEvent>,
    mut lobby_ui_query: Query<&mut LobbyUI>,
) {
    for (interaction, mut color, entity) in interaction_query.iter_mut() {
        if let Ok((
            mode_btn,
            quick_match_btn,
            create_btn,
            join_btn,
            local_btn,
            confirm_create,
            confirm_join,
            room_id_btn,
            start_btn,
            leave_btn,
            back_btn,
        )) = button_types.get(entity)
        {
            match *interaction {
                Interaction::Pressed => {
                    if let Some(mode_button) = mode_btn {
                        lobby_events.write(LobbyEvent::SelectMode(mode_button.0.clone()));
                        *color = BackgroundColor(Color::srgb(0.4, 0.7, 0.4));
                    } else if quick_match_btn.is_some() {
                        info!("🎯 Starting quick match...");
                        // Trigger real BevyGap matchmaking via StartMatchmaking event
                        if let Ok(mut lobby_ui) = lobby_ui_query.single_mut() {
                            lobby_ui.is_searching = true;
                        }
                        lobby_events.write(LobbyEvent::StartMatchmaking);
                        *color = BackgroundColor(Color::srgb(0.5, 0.1, 0.5));
                    } else if create_btn.is_some() {
                        info!("🏠 Creating room...");
                        lobby_events.write(LobbyEvent::CreateRoom);
                        *color = BackgroundColor(Color::srgb(0.1, 0.5, 0.1));
                    } else if join_btn.is_some() {
                        info!("🚪 Requesting room list from server...");
                        // Request real rooms from server instead of using dummy data
                        lobby_events.write(LobbyEvent::RequestRoomList);
                        *color = BackgroundColor(Color::srgb(0.1, 0.3, 0.5));
                    } else if local_btn.is_some() {
                        info!("🎮 Starting local game...");
                        lobby_events.write(LobbyEvent::StartLocalGame);
                        *color = BackgroundColor(Color::srgb(0.5, 0.3, 0.1));
                    } else if confirm_create.is_some() {
                        lobby_events.write(LobbyEvent::ConfirmCreateRoom);
                        *color = BackgroundColor(Color::srgb(0.1, 0.5, 0.1));
                    } else if confirm_join.is_some() {
                        if let Ok(mut lobby_ui) = lobby_ui_query.single_mut() {
                            if !lobby_ui.room_id.is_empty() {
                                lobby_ui.is_host = false;
                                lobby_ui.lobby_mode = LobbyMode::InRoom;
                                lobby_ui.is_searching = false;
                                info!("🚪 Joined room: {}", lobby_ui.room_id);
                            }
                        }
                        *color = BackgroundColor(Color::srgb(0.1, 0.3, 0.5));
                    } else if let Some(room_id_btn) = room_id_btn {
                        if let Ok(mut lobby_ui) = lobby_ui_query.single_mut() {
                            lobby_ui.room_id = room_id_btn.0.clone();
                            info!("🔤 Selected room ID: {}", lobby_ui.room_id);
                        }
                        *color = BackgroundColor(Color::srgb(0.2, 0.2, 0.2));
                    } else if start_btn.is_some() {
                        info!("🚀 Starting matchmaking...");
                        lobby_events.write(LobbyEvent::StartMatchmaking);
                        *color = BackgroundColor(Color::srgb(0.1, 0.5, 0.1));
                    } else if leave_btn.is_some() {
                        info!("👋 Leaving room...");
                        lobby_events.write(LobbyEvent::LeaveRoom);
                        *color = BackgroundColor(Color::srgb(0.5, 0.1, 0.1));
                    } else if back_btn.is_some() {
                        if let Ok(mut lobby_ui) = lobby_ui_query.single_mut() {
                            lobby_ui.lobby_mode = LobbyMode::Main;
                        }
                        *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
                    }
                }

                Interaction::Hovered => {
                    // Lighter colors on hover
                    if mode_btn.is_some() {
                        *color = BackgroundColor(Color::srgb(0.5, 0.8, 0.5));
                    } else if create_btn.is_some() {
                        *color = BackgroundColor(Color::srgb(0.3, 0.7, 0.3));
                    } else if join_btn.is_some() {
                        *color = BackgroundColor(Color::srgb(0.3, 0.5, 0.7));
                    } else if local_btn.is_some() {
                        *color = BackgroundColor(Color::srgb(0.7, 0.5, 0.3));
                    } else {
                        *color = BackgroundColor(Color::srgb(0.5, 0.5, 0.5));
                    }
                }

                Interaction::None => {
                    // Reset to normal colors
                    if let Some(mode_button) = mode_btn {
                        if let Ok(lobby_ui) = lobby_ui_query.single() {
                            if mode_button.0 == lobby_ui.selected_mode {
                                *color = BackgroundColor(Color::srgb(0.4, 0.7, 0.4));
                            } else {
                                *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
                            }
                        }
                    } else if create_btn.is_some() {
                        *color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2));
                    } else if join_btn.is_some() {
                        *color = BackgroundColor(Color::srgb(0.2, 0.4, 0.6));
                    } else if local_btn.is_some() {
                        *color = BackgroundColor(Color::srgb(0.6, 0.4, 0.2));
                    } else if confirm_create.is_some() {
                        *color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2));
                    } else if confirm_join.is_some() {
                        *color = BackgroundColor(Color::srgb(0.2, 0.4, 0.6));
                    } else if room_id_btn.is_some() {
                        *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
                    } else if start_btn.is_some() {
                        *color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2));
                    } else if leave_btn.is_some() {
                        *color = BackgroundColor(Color::srgb(0.6, 0.2, 0.2));
                    } else if back_btn.is_some() {
                        *color = BackgroundColor(Color::srgb(0.4, 0.4, 0.4));
                    }
                }
            }
        }
    }
}

// Simple lobby UI update (just update player count in room)
fn update_simple_ui(
    lobby_ui_query: Query<&LobbyUI>,
    mut player_count_query: Query<&mut Text, With<PlayerCountText>>,
) {
    if let (Ok(lobby_ui), Ok(mut text)) = (lobby_ui_query.single(), player_count_query.single_mut())
    {
        if lobby_ui.lobby_mode == LobbyMode::InRoom {
            **text = format!("Players: {}/4", lobby_ui.current_players);
        }
    }
}

// 🎯 Handle lobby events
fn handle_lobby_events(
    mut lobby_events: EventReader<LobbyEvent>,
    mut lobby_ui_query: Query<&mut LobbyUI>,
    mut next_state: ResMut<NextState<AppState>>,
    mut room_registry: ResMut<ClientRoomRegistry>,
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
                info!(
                    "🎮 Player joined! Current players: {}",
                    lobby_ui.current_players
                );
            }
            LobbyEvent::PlayerLeft(player_count) => {
                lobby_ui.current_players = *player_count;
                info!(
                    "👋 Player left! Current players: {}",
                    lobby_ui.current_players
                );
            }
            LobbyEvent::StartGame => {
                info!(
                    "🚀 Starting multiplayer game with {} players!",
                    lobby_ui.current_players
                );
                lobby_ui.is_searching = false;
                next_state.set(AppState::InGame);
            }
            LobbyEvent::StartMatchmaking => {
                info!("🔍 Starting matchmaking...");
                lobby_ui.is_searching = true;
                // Real BevyGap matchmaking will be handled by handle_matchmaking_events
            }
            LobbyEvent::StartLocalGame => {
                info!("🎮 Starting local game!");
                next_state.set(AppState::InGame);
            }
            LobbyEvent::SelectMode(mode) => {
                lobby_ui.selected_mode = mode.clone();
                info!("🎯 Selected game mode: {}", mode);
            }
            LobbyEvent::CreateRoom => {
                lobby_ui.lobby_mode = LobbyMode::CreateRoom;
                info!("🏠 Switching to create room mode");
            }
            LobbyEvent::ConfirmCreateRoom => {
                // Generate room ID using random numbers for uniqueness (WASM-compatible)
                let mut rng = rand::thread_rng();
                let room_num = rng.gen_range(1..=999);
                let room_id = format!("ROOM{:03}", room_num);

                // Create room info and add to registry
                let room_info = RoomInfo {
                    room_id: room_id.clone(),
                    current_players: 1,
                    max_players: 4,
                    host_name: lobby_ui.player_name.clone(),
                    game_mode: lobby_ui.selected_mode.clone(),
                };

                room_registry.rooms.push(room_info);

                lobby_ui.room_id = room_id;
                lobby_ui.is_host = true;
                lobby_ui.lobby_mode = LobbyMode::InRoom;
                lobby_ui.is_searching = false;
                info!("🏠 Created room: {}", lobby_ui.room_id);
            }
            LobbyEvent::JoinRoom => {
                lobby_ui.lobby_mode = LobbyMode::JoinRoom;
                info!("🚪 Switching to join room mode");
            }
            LobbyEvent::RequestRoomList => {
                info!("📋 Requesting room list from server...");
                // Combine registry rooms with some default rooms for testing
                let mut available_rooms = room_registry.rooms.clone();

                // Add some default test rooms if the registry is empty
                if available_rooms.is_empty() {
                    available_rooms = vec![
                        RoomInfo {
                            room_id: "ROOM001".to_string(),
                            current_players: 2,
                            max_players: 4,
                            host_name: "Player1".to_string(),
                            game_mode: "casual".to_string(),
                        },
                        RoomInfo {
                            room_id: "ROOM002".to_string(),
                            current_players: 1,
                            max_players: 4,
                            host_name: "Player2".to_string(),
                            game_mode: "ranked".to_string(),
                        },
                    ];
                }

                lobby_ui.available_rooms = available_rooms;
                lobby_ui.lobby_mode = LobbyMode::JoinRoom;
            }
            LobbyEvent::RoomListReceived(rooms) => {
                info!("📋 Received {} rooms from server", rooms.len());
                lobby_ui.available_rooms = rooms.clone();
                lobby_ui.lobby_mode = LobbyMode::JoinRoom;
            }
            LobbyEvent::EnterRoomId(room_id) => {
                lobby_ui.room_id = room_id.clone();
                info!("🔤 Entered room ID: {}", room_id);
            }
            LobbyEvent::LeaveRoom => {
                lobby_ui.lobby_mode = LobbyMode::Main;
                lobby_ui.room_id.clear();
                lobby_ui.is_host = false;
                lobby_ui.current_players = 1;
                lobby_ui.is_searching = false;
                info!("👋 Left room, returning to main lobby");
            }
            LobbyEvent::LobbyCreated(lobby_name) => {
                info!("🏠 Lobby created: {}", lobby_name);
                // Continue showing searching status while deploying
            }
            #[cfg(feature = "bevygap")]
            LobbyEvent::LobbyDeployed(lobby_response) => {
                info!(
                    "🚀 Lobby deployed successfully! Server URL: {}",
                    lobby_response.url
                );
                lobby_ui.is_searching = false;
                // Connection will be handled by BevyGap
            }
            LobbyEvent::LobbyDeploymentFailed(error) => {
                error!("❌ Lobby deployment failed: {}", error);
                lobby_ui.is_searching = false;
            }
            LobbyEvent::ConnectedToServer => {
                info!("🎮 Connected to game server!");
                lobby_ui.is_searching = false;
                next_state.set(AppState::InGame);
            }
        }
    }
}

// Handle bevygap connection events to transition from lobby to game
fn handle_connection_events() {
    // TODO: Listen for actual BevyGap connection success/failure events here
    // For now, the real connection handling is done via LobbyEvent::ConnectedToServer
    // in handle_lobby_events, so this function is currently a placeholder
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
        "ws://voidloop.quest:3000/matchmaker/ws".to_string()
    }
}

// 🏷️ UI component markers
#[derive(Component)]
struct PlayerCountText;

#[derive(Component)]
struct LobbyContainer;

#[derive(Component)]
struct LobbyUIElements;

#[derive(Component)]
struct ModeButton(String);

#[derive(Component)]
struct QuickMatchButton;

#[derive(Component)]
struct CreateRoomButton;

#[derive(Component)]
struct JoinRoomButton;

#[derive(Component)]
struct LocalPlayButton;

#[derive(Component)]
struct ConfirmCreateButton;

#[derive(Component)]
struct ConfirmJoinButton;

#[derive(Component)]
struct RoomIdButton(String);

#[derive(Component)]
struct StartGameButton;

#[derive(Component)]
struct LeaveRoomButton;

#[derive(Component)]
struct BackButton;

// 🎯 Handle real matchmaking with Edgegap integration
#[cfg(feature = "bevygap")]
fn handle_matchmaking_events(
    mut lobby_events: EventReader<LobbyEvent>,
    mut lobby_events_writer: EventWriter<LobbyEvent>,
    mut commands: Commands,
    lobby_config: Res<LobbyConfig>,
    mut lobby_state: ResMut<EdgegapLobbyState>,
    lobby_ui_query: Query<&LobbyUI>,
) {
    for event in lobby_events.read() {
        if let LobbyEvent::StartMatchmaking = event {
            if let Ok(lobby_ui) = lobby_ui_query.single() {
                let api_token = if let Some(token) = &lobby_config.edgegap_token {
                    token.clone()
                } else {
                    error!(
                        "🚫 EDGEGAP_TOKEN not configured! Set environment variable EDGEGAP_TOKEN"
                    );
                    lobby_events_writer.write(LobbyEvent::LobbyDeploymentFailed(
                        "EDGEGAP_TOKEN not configured".to_string(),
                    ));
                    return;
                };

                let base_url = lobby_config.edgegap_api_url.clone();
                let selected_mode = lobby_ui.selected_mode.clone();

                // Generate unique lobby name using random ID (WASM-compatible)
                let mut rng = rand::thread_rng();
                let random_id = rng.gen_range(10000..99999);
                let lobby_name = format!(
                    "voidloop-{}-{}",
                    selected_mode,
                    random_id
                );

                info!("🔧 Creating Edgegap lobby: {}", lobby_name);

                // Configure Edgegap API client
                let mut cfg = Configuration::default();
                cfg.base_path = base_url;
                cfg.api_key = Some(edgegap_async::apis::configuration::ApiKey {
                    prefix: Some("Bearer".into()),
                    key: api_token,
                });

                // Handle async operations differently for native vs WASM
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    let lobby_name_clone = lobby_name.clone();
                    let create_result = runtime.block_on(async {
                        create_and_deploy_lobby(&cfg, lobby_name_clone).await
                    });

                    handle_lobby_creation_result(
                        create_result,
                        &mut lobby_state,
                        &mut lobby_events_writer,
                        &mut commands,
                    );
                }

                #[cfg(target_arch = "wasm32")]
                {
                    // For WASM, spawn the async task without blocking
                    let lobby_name_clone = lobby_name.clone();
                    
                    // Note: In WASM, we can't directly write to the event writer from the async context
                    // This is a limitation - in a real implementation, you'd need to use channels or
                    // a different architecture to communicate results back to the Bevy system
                    spawn_local(async move {
                        let result = create_and_deploy_lobby(&cfg, lobby_name_clone).await;
                        
                        // Log the result - in a real implementation, you'd send this through a channel
                        // or use a different mechanism to communicate back to the Bevy system
                        match result {
                            Ok((created_name, deploy_response)) => {
                                info!("✅ WASM: Lobby created and deployed: {}", created_name);
                                info!("📍 WASM: Server URL: {}", deploy_response.url);
                                // TODO: Send success event back to Bevy system
                            }
                            Err(error_msg) => {
                                error!("❌ WASM: Failed to create/deploy lobby: {}", error_msg);
                                // TODO: Send error event back to Bevy system
                            }
                        }
                    });

                    // For now, immediately set state to indicate we started the process
                    lobby_state.is_deploying = true;
                    info!("🔄 WASM: Lobby creation started in background...");
                }
            }
        }
    }
}

// Helper function to handle the async lobby creation and deployment
#[cfg(feature = "bevygap")]
async fn create_and_deploy_lobby(
    cfg: &Configuration,
    lobby_name: String,
) -> Result<(String, LobbyReadResponse), String> {
    // Create lobby
    let payload = LobbyCreatePayload::new(lobby_name.clone());
    let create_result = lobbies_api::lobby_create(cfg, payload).await;

    match create_result {
        Ok(create_response) => {
            info!("✅ Lobby created: {}", create_response.name);

            // Deploy the lobby (this starts the game server)
            let deploy_payload = LobbyDeployPayload {
                name: create_response.name.clone(),
            };
            let deploy_result = lobbies_api::lobby_deploy(cfg, deploy_payload).await;

            match deploy_result {
                Ok(deploy_response) => {
                    info!("🚀 Lobby deployed successfully!");
                    info!("📍 Server URL: {}", deploy_response.url);
                    info!("📊 Status: {}", deploy_response.status);
                    Ok((create_response.name, deploy_response))
                }
                Err(e) => {
                    error!("❌ Failed to deploy lobby: {:?}", e);
                    Err(format!("Deploy failed: {:?}", e))
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to create lobby: {:?}", e);
            Err(format!("Create failed: {:?}", e))
        }
    }
}

// Helper function to handle the lobby creation result
#[cfg(all(feature = "bevygap", not(target_arch = "wasm32")))]
fn handle_lobby_creation_result(
    create_result: Result<(String, LobbyReadResponse), String>,
    lobby_state: &mut ResMut<EdgegapLobbyState>,
    lobby_events_writer: &mut EventWriter<LobbyEvent>,
    commands: &mut Commands,
) {
    match create_result {
        Ok((created_name, deploy_response)) => {
            lobby_state.lobby_name = Some(created_name.clone());
            lobby_state.lobby_response = Some(deploy_response.clone());
            lobby_state.is_deploying = false;

            lobby_events_writer.write(LobbyEvent::LobbyCreated(created_name));
            lobby_events_writer.write(LobbyEvent::LobbyDeployed(deploy_response));

            // Now attempt to connect via BevyGap
            info!("🔗 Attempting BevyGap connection...");
            commands.bevygap_connect_client();

            // Send connection success after a brief delay (in real implementation,
            // this would listen for actual BevyGap connection events)
            lobby_events_writer.write(LobbyEvent::ConnectedToServer);
        }
        Err(error_msg) => {
            lobby_state.deployment_error = Some(error_msg.clone());
            lobby_state.is_deploying = false;
            lobby_events_writer.write(LobbyEvent::LobbyDeploymentFailed(error_msg));
        }
    }
}

// ==== PLACEHOLDER FOR FUTURE NETWORKING FEATURES ====
// TODO: Add room message handling when networking integration is complete
// ==== END PLACEHOLDER ====
