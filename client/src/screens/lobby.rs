use bevy::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[cfg(feature = "bevygap")]
use bevygap_client_plugin::prelude::{BevygapConnectExt, BevygapClientState};

use shared::RoomInfo;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::{RequestInit, RequestMode};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Resource, Default)]
pub struct ClientRoomRegistry {
    pub rooms: Vec<RoomInfo>,
}

#[derive(Resource, Default)]
pub struct ConnectionState {
    // Reserved for future connection state tracking
}

#[derive(Resource, Clone, Debug)]
pub struct LobbyConfig {
#[cfg(target_arch = "wasm32")]
thread_local! {
    static PENDING_ROOM_CREATED: RefCell<Option<RoomInfo>> = RefCell::new(None);
    static PENDING_ROOM_LIST: RefCell<Option<Vec<RoomInfo>>> = RefCell::new(None);
    static PENDING_NOTICE: RefCell<Option<String>> = RefCell::new(None);
}

#[derive(Resource, Default)]
pub struct UiNotice { pub msg: Option<String>, pub timer: f32 }

#[derive(Component)]
struct NoticeText;

    pub domain: String,           // "voidloop.quest"
    pub matchmaker_url: String,   // "wss://voidloop.quest/matchmaker/ws"
    pub max_players: u32,         // 4
    pub lobby_modes: Vec<String>, // ["casual", "ranked", "custom"]
}

impl Default for LobbyConfig {
    fn default() -> Self {
        Self {
            domain: "voidloop.quest".to_string(),
            matchmaker_url: get_matchmaker_url(),
            max_players: 4,
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
// Helper: server-side lobby room representation from bevygap httpd
#[derive(Deserialize, Debug, Clone)]
#[cfg(target_arch = "wasm32")]
struct ServerLobbyRoom {
    id: String,
    host_name: String,
    game_mode: String,
    created_at: u64,
    started: bool,
    current_players: u32,
    max_players: u32,
}

    PlayerJoined(u32),
    PlayerLeft(u32),
    StartGame,
    StartLocalGame,
    SelectMode(String),
            LobbyEvent::StartGame => {
                // Host starts room: ask server to mark room started; on success, begin matchmaking
                #[cfg(target_arch = "wasm32")]
                {
                    if lobby_ui.room_id.is_empty() {
                        web_sys::console::warn_1(&"No room id to start".into());
                    } else {
                        let room_id = lobby_ui.room_id.clone();
                        spawn_local(async move {
                            let url = format!("{}/lobby/api/rooms/{}/start", http_base(), room_id);
                            match fetch_json(&url, "POST", None).await {
                                Ok(resp) => {
                                    let resp: web_sys::Response = resp.dyn_into().unwrap();
                                    if !resp.ok() {
                                        web_sys::console::error_1(&format!("Failed to start room, status {}", resp.status()).into());
                                    }
                                }
                                Err(e) => web_sys::console::error_1(&e),
                            }
                        });
                        // Immediately trigger BevyGap matchmaking; server will spin up via Edgegap
                        commands.bevygap_connect_client();
                        lobby_ui.is_searching = true;
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
            .insert_resource(UiNotice::default())
            .add_systems(Update, (pump_async_results, show_notice).run_if(in_state(AppState::Lobby)))

                    // Native: just begin matchmaking
                    commands.bevygap_connect_client();
                    lobby_ui.is_searching = true;
                }
            }
            LobbyEvent::LeaveRoom => {
                #[cfg(target_arch = "wasm32")]
                {
                    if !lobby_ui.room_id.is_empty() {
                        let room_id = lobby_ui.room_id.clone();
                        let player_name = lobby_ui.player_name.clone();
                        spawn_local(async move {
                            let url = format!("{}/lobby/api/rooms/{}/leave", http_base(), room_id);
                            #[derive(Serialize)]
                            struct LeaveReq<'a> { player_name: &'a str }
                            let body = serde_json::to_string(&LeaveReq { player_name: &player_name }).unwrap();
                            match fetch_json(&url, "POST", Some(body)).await {
                                Ok(resp) => {
                                    let resp: web_sys::Response = resp.dyn_into().unwrap();
                                    if !resp.ok() {
                                        web_sys::console::error_1(&format!("Failed to leave room, status {}", resp.status()).into());
                                    }
                                }
                                Err(e) => web_sys::console::error_1(&e),
                            }
                        });
                    }
                }
                // Reset UI locally
                lobby_ui.lobby_mode = LobbyMode::Main;
                lobby_ui.room_id.clear();
                lobby_ui.is_host = false;
                lobby_ui.current_players = 1;
                lobby_ui.is_searching = false;
                info!("👋 Left room, returning to main lobby");
            }

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
fn show_notice(
    mut cmds: Commands,
    mut notice: ResMut<UiNotice>,
    time: Res<Time>,
    mut q_text: Query<Entity, With<NoticeText>>,
) {
    // display current notice text if any
    let mut need_spawn = false;
    if notice.msg.is_some() && q_text.is_empty() {
        need_spawn = true;
    }
    if need_spawn {
        let e = cmds
            .spawn((
                NoticeText,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(8.0),
                    right: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.8)),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new(notice.msg.clone().unwrap_or_default()),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.8, 0.2)),
                ));
            })
            .id();
        let _ = e; // keep created
        notice.timer = 3.0; // show for 3 seconds
    }
    if let Some(msg) = &notice.msg {
        if notice.timer > 0.0 {
            notice.timer -= time.delta_seconds();
        } else {
            notice.msg = None;
            for e in q_text.iter_mut() {
                cmds.entity(e).despawn_recursive();
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn pump_async_results(
    mut notice: ResMut<UiNotice>,
    mut lobby_q: Query<&mut LobbyUI>,
) {
    // room created
    PENDING_ROOM_CREATED.with(|cell| {
        if let Some(room) = cell.borrow_mut().take() {
            if let Ok(mut ui) = lobby_q.get_single_mut() {
                ui.room_id = room.room_id.clone();
                ui.is_host = true;
                ui.lobby_mode = LobbyMode::InRoom;
                ui.is_searching = false;
            }
        }
    });
    // room list
    PENDING_ROOM_LIST.with(|cell| {
        if let Some(list) = cell.borrow_mut().take() {
            if let Ok(mut ui) = lobby_q.get_single_mut() {
                ui.available_rooms = list;
                ui.lobby_mode = LobbyMode::JoinRoom;
            }
        }
    });
    // notices
    PENDING_NOTICE.with(|cell| {
        if let Some(msg) = cell.borrow_mut().take() {
            notice.msg = Some(msg);
            notice.timer = 0.0; // cause spawn next frame
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn http_base() -> String {
    // Build http(s) base from current location
    let window = web_sys::window().expect("no window");
    let loc = window.location();
    let protocol = loc.protocol().unwrap_or_else(|_| "http:".into());
    let scheme = if protocol == "https:" { "https" } else { "http" };
    let host = loc.host().unwrap();
    format!("{}://{}", scheme, host)
}

#[cfg(target_arch = "wasm32")]
fn fetch_json(url: &str, method: &str, body: Option<String>) -> wasm_bindgen_futures::JsFuture {
    use wasm_bindgen::JsValue;

    let mut opts = RequestInit::new();
    opts.method(method);
    opts.mode(RequestMode::Cors);
    if let Some(b) = body {
        opts.body(Some(&JsValue::from_str(&b)));
    }

    let request = web_sys::Request::new_with_str_and_init(url, &opts).unwrap();
    request
        .headers()
        .set("Content-Type", "application/json")
        .unwrap();

    let window = web_sys::window().unwrap();
    wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
}

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
            LobbyEvent::RequestRoomList => {
                info!("📋 Requesting room list from server...");
                #[cfg(target_arch = "wasm32")]
                {
                    let mut ui_ptr = lobby_ui.clone();
                    // fire async fetch to /lobby/api/rooms
                    spawn_local(async move {
                        let url = format!("{}/lobby/api/rooms", http_base());
                        match fetch_json(&url, "GET", None).await {
                            Ok(resp) => {
                                let resp: web_sys::Response = resp.dyn_into().unwrap();
                                match wasm_bindgen_futures::JsFuture::from(resp.json().unwrap()).await {
                                    Ok(js) => {
                                        let rooms: Vec<ServerLobbyRoom> = serde_wasm_bindgen::from_value(js).unwrap_or_default();
                                        let list: Vec<RoomInfo> = rooms
                                            .into_iter()
                                            .filter(|r| !r.started)
                                            .map(|r| RoomInfo { room_id: r.id, current_players: r.current_players, max_players: r.max_players, host_name: r.host_name, game_mode: r.game_mode })
                                            .collect();
                                        PENDING_ROOM_LIST.with(|cell| cell.replace(Some(list)));
                                    }
                                    Err(e) => {
                                        PENDING_NOTICE.with(|cell| cell.replace(Some(format!("Failed loading rooms: {e:?}"))));
                                    }
                                }
                            }
                            Err(e) => PENDING_NOTICE.with(|cell| cell.replace(Some(format!("Failed http rooms: {e:?}")))),
                        }
                    });
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // Native fallback - still use local registry
                    let mut available_rooms = room_registry.rooms.clone();
                    if available_rooms.is_empty() {
                        available_rooms = vec![
                            RoomInfo { room_id: "ROOM001".into(), current_players: 2, max_players: 4, host_name: "Player1".into(), game_mode: "casual".into() },
                            RoomInfo { room_id: "ROOM002".into(), current_players: 1, max_players: 4, host_name: "Player2".into(), game_mode: "ranked".into() },
                        ];
                    }
                    lobby_ui.available_rooms = available_rooms;
                    lobby_ui.lobby_mode = LobbyMode::JoinRoom;
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
    mut commands: Commands,
    #[cfg(feature = "bevygap")]
    lobby_config: Option<Res<LobbyConfig>>,
    #[cfg(feature = "bevygap")]
    mut lobby_state: Option<ResMut<EdgegapLobbyState>>,
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
                
                // Handle real BevyGap matchmaking inline
                #[cfg(feature = "bevygap")]
                {
                    // Get required resources for matchmaking
                    if let (Some(lobby_config), Some(mut lobby_state)) = (lobby_config.as_deref(), lobby_state.as_mut()) {
                        let api_token = if let Some(token) = &lobby_config.edgegap_token {
                            token.clone()
                        } else {
                            let error_msg = "EDGEGAP_TOKEN not configured".to_string();
                            error!(
                                "🚫 EDGEGAP_TOKEN not configured! Set environment variable EDGEGAP_TOKEN"
                            );
                            // Send error event for next frame
                            commands.queue(move |world: &mut World| {
                                world.send_event(LobbyEvent::LobbyDeploymentFailed(error_msg));
                            });
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
                #[cfg(target_arch = "wasm32")]
                {
                    let player_name = lobby_ui.player_name.clone();
                    let game_mode = lobby_ui.selected_mode.clone();
                    spawn_local(async move {
                        let url = format!("{}/lobby/api/rooms", http_base());
                        #[derive(Serialize)]
                        struct CreateReq<'a> { host_name: &'a str, game_mode: &'a str, max_players: u32 }
                        let body = serde_json::to_string(&CreateReq { host_name: &player_name, game_mode: &game_mode, max_players: 4 }).unwrap();
                        match fetch_json(&url, "POST", Some(body)).await {
                            Ok(resp) => {
                                let resp: web_sys::Response = resp.dyn_into().unwrap();
                                if !resp.ok() {
                                    let status = resp.status();
                                    web_sys::console::error_1(&format!("Create room failed http {}", status).into());
                                    return;
                                }
                                match wasm_bindgen_futures::JsFuture::from(resp.json().unwrap()).await {
                                    Ok(js) => {
                                        let room: ServerLobbyRoom = serde_wasm_bindgen::from_value(js).unwrap();
                                        web_sys::console::log_1(&format!("Room created {}", room.id).into());
                                    }
                                    Err(e) => web_sys::console::error_1(&e),
                                }
                            }
                            Err(e) => web_sys::console::error_1(&e),
                        }
                    });
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // Keep local fallback for native
                    let mut rng = rand::thread_rng();
                    let room_num = rng.gen_range(1..=999);
                    let room_id = format!("ROOM{:03}", room_num);
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
    commands: &mut Commands,
) {
    match create_result {
        Ok((created_name, deploy_response)) => {
            lobby_state.lobby_name = Some(created_name.clone());
            lobby_state.lobby_response = Some(deploy_response.clone());
            lobby_state.is_deploying = false;

            let created_name_clone = created_name.clone();
            let deploy_response_clone = deploy_response.clone();
            commands.queue(move |world: &mut World| {
                world.send_event(LobbyEvent::LobbyCreated(created_name_clone));
                world.send_event(LobbyEvent::LobbyDeployed(deploy_response_clone));
                world.send_event(LobbyEvent::ConnectedToServer);
            });

            // Now attempt to connect via BevyGap
            info!("🔗 Attempting BevyGap connection...");
            commands.bevygap_connect_client();
        }
        Err(error_msg) => {
            lobby_state.deployment_error = Some(error_msg.clone());
            lobby_state.is_deploying = false;
            let error_msg_clone = error_msg.clone();
            commands.queue(move |world: &mut World| {
                world.send_event(LobbyEvent::LobbyDeploymentFailed(error_msg_clone));
            });
        }
    }
}

// ==== PLACEHOLDER FOR FUTURE NETWORKING FEATURES ====
// TODO: Add room message handling when networking integration is complete
// ==== END PLACEHOLDER ====
