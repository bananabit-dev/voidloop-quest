use bevy::prelude::*;
use bevy::{color::palettes::css, prelude::*};
#[cfg(feature = "bevygap")]
use bevygap_client_plugin::prelude::*;

use crate::screens::Screen;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Lobby), spawn_lobby_screen);
    app.add_systems(Update, lobby_button_system.run_if(in_state(Screen::Lobby)));
}

#[derive(Component)]
struct LobbyUIButton;
#[derive(Component)]
struct LobbyUIText;

#[derive(Resource, Clone, Copy, Debug)]
pub struct DesiredPlayerLimit(pub u8);

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn spawn_lobby_screen(mut commands: Commands) {
    commands.insert_resource(DesiredPlayerLimit(4));

    commands
        .spawn((
            StateScoped(Screen::Lobby),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((LobbyUIText, Text::default(), TextFont::from_font_size(26.0)))
                .with_children(|p| {
                    p.spawn(TextSpan::new("Lobby"));
                });

            // Player limit buttons 1..=4
            parent
                .spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(10.0), ..default() })
                .with_children(|row| {
                    for n in 1..=4u8 {
                        row
                            .spawn((LobbyUIButton, Button, Node { width: Val::Px(60.0), height: Val::Px(40.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, BackgroundColor(NORMAL_BUTTON), BorderRadius::MAX, BorderColor(Color::BLACK)))
                            .insert(Name::new(format!("limit-{n}")))
                            .with_children(|p| { 
                                p.spawn((Text::default(), TextFont::from_font_size(18.0))); 
                            });
                        // add text span to the last spawned entity (the Node added above)
                    }
                });

            // Create/Find/Join buttons
            for label in ["Create", "Search", "Join"] { 
                parent
                    .spawn((LobbyUIButton, Button, Node { width: Val::Px(180.0), height: Val::Px(50.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, BackgroundColor(NORMAL_BUTTON), BorderRadius::MAX, BorderColor(Color::BLACK)))
                    .insert(Name::new(label))
                    .with_children(|p| { p.spawn((Text::default(), TextFont::from_font_size(20.0))); });
            }
        });
}

fn lobby_button_system(
    mut q: Query<(&Interaction, &mut BackgroundColor, &Name), (Changed<Interaction>, With<LobbyUIButton>)>,
    mut desired_limit: ResMut<DesiredPlayerLimit>,
    mut next_screen: ResMut<NextState<Screen>>, 
    #[cfg(feature = "bevygap")] mut config: ResMut<BevygapClientConfig>,
) {
    for (interaction, mut bg, name) in &mut q {
        match *interaction {
            Interaction::Pressed => {
                *bg = PRESSED_BUTTON.into();
                if name.as_str().starts_with("limit-") {
                    if let Some(n) = name.as_str().strip_prefix("limit-").and_then(|s| s.parse::<u8>().ok()) {
                        desired_limit.0 = n.clamp(1, 4);
                        info!("Player limit set to {}", desired_limit.0);
                    }
                } else if name.as_str() == "Create" {
                    // For now, transition to Connect and request a session. The matchmaker will interpret player_limit.
                    #[cfg(feature = "bevygap")]
                    {
                        config.fake_client_ip = config.fake_client_ip.clone();
                    }
                    next_screen.set(Screen::Connect);
                } else if name.as_str() == "Search" {
                    // Placeholder: would query available sessions/lobbies via HTTP/NATS.
                    info!("Search clicked (not yet implemented)");
                } else if name.as_str() == "Join" {
                    next_screen.set(Screen::Connect);
                }
            }
            Interaction::Hovered => { *bg = HOVERED_BUTTON.into(); }
            Interaction::None => { *bg = NORMAL_BUTTON.into(); }
        }
    }
}
