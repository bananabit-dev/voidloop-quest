use crate::screens::*;
#[cfg(feature = "bevygap")]
use bevygap_client_plugin::BevygapClientConfig;

use bevy::prelude::*;
use bevy::{color::palettes::css, prelude::*};
use lightyear::prelude::*;
use shared::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(update_connect_status_text_observer);
    app.add_systems(OnEnter(Screen::Connect), spawn_connect_screen);
    // systems that only run in Connect state.
    app.add_systems(
        Update,
        (
            continue_to_gameplay_screen.run_if(connected_to_server),
            button_system,
        )
            .run_if(in_state(Screen::Connect)),
    );
    #[cfg(feature = "bevygap")]
    app.add_systems(
        Update,
        on_bevygap_state_change.run_if(in_state(Screen::Connect)),
    );
}

fn continue_to_gameplay_screen(mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Gameplay);
}

fn connected_to_server(connected: Option<Single<&Connected>>) -> bool {
    connected.is_some()
}

// Marker tag for loading screen components.
#[derive(Component)]
struct ConnectUIText;
#[derive(Component)]
struct ConnectUIButton;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn spawn_connect_screen(mut commands: Commands, _asset_server: ResMut<AssetServer>, #[cfg(feature = "bevygap")] desired: Option<Res<super::lobby::DesiredPlayerLimit>>, #[cfg(feature = "bevygap")] mut cfg: ResMut<BevygapClientConfig>) {
    info!("spawn_connect_screen");

    // If we arrived here from Lobby with a desired player limit, carry it over into the request
    #[cfg(feature = "bevygap")]
    if let Some(desired) = desired { 
        // Carry over desired player limit via env var; BevygapClientPlugin includes it in request
        std::env::set_var("VOIDLOOP_PLAYER_LIMIT", desired.0.to_string());
    }

    commands
        .spawn((
            StateScoped(Screen::Connect),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    ConnectUIButton,
                    Button,
                    // visual style of the button is expressed via Node properties + colors
                    Node {
                        width: Val::Px(150.0),
                        height: Val::Px(65.0),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect { bottom: Val::Px(20.0), ..default() },
                        ..default()
                    },
                    BorderColor(Color::BLACK),
                    BorderRadius::MAX,
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_children(|parent| {
                    parent
                        .spawn((Text::default(), TextFont::from_font_size(22.0), TextColor(Color::srgb(0.9,0.9,0.9)),))
                        .with_children(|p|{
                            p.spawn(TextSpan::new("Connect"));
                        });
                });

            parent
                .spawn((ConnectUIText, Text::default(), TextFont::from_font_size(30.0)))
                .with_children(|p| {
                    p.spawn(TextSpan::new("Standing By"));
                });
        });
}

#[derive(Event, Clone)]
struct ConnectStatusText(String);

/// Emitted when user clicks the connect button.
#[derive(Event)]
pub(crate) struct ConnectToServerRequest;

fn update_connect_status_text_observer(
    trigger: Trigger<ConnectStatusText>,
    q_roots: Query<&Children, With<ConnectUIText>>,
    mut q_spans: Query<&mut TextSpan>,
) {
    if let Ok(children) = q_roots.single() {
        for child in children.iter() {
                if let Ok(mut span) = q_spans.get_mut(child) {
                span.0.clone_from(&trigger.event().0);
                break;
            }
        }
    }
}

#[cfg(feature = "bevygap")]
fn on_bevygap_state_change(
    state: Res<State<bevygap_client_plugin::BevygapClientState>>,
    mut commands: Commands,
) {
    use bevygap_client_plugin::BevygapClientState;

    let msg = match state.get() {
        BevygapClientState::Dormant => "Chrome only atm!".to_string(),
        BevygapClientState::Request => "Making request...".to_string(),
        BevygapClientState::AwaitingResponse(msg) => msg.clone(),
        BevygapClientState::ReadyToConnect => "Ready!".to_string(),
        BevygapClientState::Finished => "Finished connection setup.".to_string(),
        BevygapClientState::Error(code, msg) => format!("ERR {code}: {msg}"),
    };
    commands.trigger(ConnectStatusText(msg));
}

fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Changed<Interaction>, With<ConnectUIButton>),
    >,
    mut commands: Commands,
) {
    for (interaction, mut color, mut border_color, _children) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.0 = css::RED.into();
                info!("PRESSED");
                commands.trigger(ConnectStatusText("Connecting to server...".to_string()));
                commands.trigger(ConnectToServerRequest);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}
