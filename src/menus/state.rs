use bevy::{
    ecs::system::EntityCommands,
    hierarchy::{BuildChildren, ChildBuilder},
    math::{Size, Vec3},
    prelude::{
        AssetServer, Color, Commands, Component, EventWriter, Handle, Image, Res, ResMut, Transform,
    },
    sprite::{Sprite, SpriteBundle},
    text::{Font, HorizontalAlign, Text, Text2dBundle, TextAlignment, TextSection, TextStyle},
    transform::TransformBundle,
    utils::HashMap,
};

use crate::{tiling::*, ui::*};

use super::{events::*, RulesContainer};

pub struct MenuState {
    pub button: Handle<Image>,
    pub font: Handle<Font>,
    pub active_shape: TileShape,
    pub active_state: u32,
    pub state_to_color: HashMap<u32, Color>,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            button: Default::default(),
            font: Default::default(),
            active_shape: TileShape::Square,
            active_state: 0u32,
            state_to_color: Default::default(),
        }
    }
}

impl MenuState {
    /// Create a text bundle using the default font of the given string in the given
    /// color and size, center aligned, positioned closer to the camera by 1 unit.
    pub fn get_text_bundle(&self, text: String, size: f32, color: Color) -> Text2dBundle {
        Text2dBundle {
            text: Text {
                sections: vec![TextSection {
                    value: text,
                    style: TextStyle {
                        font: self.font.clone(),
                        font_size: size,
                        color,
                    },
                }],
                alignment: TextAlignment {
                    vertical: bevy::text::VerticalAlign::Center,
                    horizontal: bevy::text::HorizontalAlign::Center,
                },
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ..Default::default()
        }
    }

    /// Build a group of buttons attached to the given root with
    /// the passed in text, text color and event for each button in the group.
    pub fn build_button_group<Event: Component + Clone>(
        &self,
        root: &mut EntityCommands,
        background: Color,
        mut data: Vec<(String, Color, Event)>,
        width: f32,
        height: f32,
        font_size: f32,
        font_color: Color,
        margin: f32,
    ) {
        root.insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: background,
                ..Default::default()
            },
            texture: self.button.clone(),
            ..Default::default()
        })
        .insert(UiElement {
            size: Size::new(width, height),
            ..Default::default()
        })
        .with_children(|choice_builder| {
            let num = data.len() as f32;
            let width = width / num;
            for (i, (text, color, event)) in data.drain(..).enumerate() {
                choice_builder
                    .spawn_bundle(SpriteBundle {
                        sprite: Sprite {
                            color,
                            ..Default::default()
                        },
                        texture: self.button.clone(),
                        transform: Transform::from_translation(Vec3::new(
                            width * (i as f32 - (num - 1.0) * 0.5),
                            0.0,
                            1.0,
                        )),
                        ..Default::default()
                    })
                    .insert(UiElement {
                        size: Size::new(width - margin, height - margin),
                        hover_state: UiStateDetails {
                            accepts_state: true,
                            ..Default::default()
                        },
                        click_state: UiStateDetails {
                            accepts_state: true,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .insert(Button::new(self.button.clone(), event))
                    .with_children(|button_text| {
                        button_text.spawn_bundle(self.get_text_bundle(text, font_size, font_color));
                    });
            }
        });
    }

    /// Spawn a horizontally layed out group with the given label prefixing
    /// the group. We will call the passed in function with the child builder
    /// of the group
    pub fn spawn_labeled(
        &self,
        root: &mut EntityCommands,
        size: Size,
        label: String,
        label_color: Color,
        f: impl FnOnce(&Self, &mut ChildBuilder),
    ) {
        root.insert_bundle(TransformBundle::default())
            .insert(UiElement {
                size,
                ..Default::default()
            })
            .insert(UiLinearScroll {
                layout_direction: LayoutDirection::Horizontal,
                ..Default::default()
            })
            .with_children(|child_builder| {
                child_builder
                    .spawn_bundle(self.get_text_bundle(
                        label,
                        super::REGULAR_FONT_SIZE,
                        label_color,
                    ))
                    .insert(UiElement {
                        size: Size::new(100.0, size.height - super::REGULAR_MARGIN),
                        ..Default::default()
                    });
                f(self, child_builder);
            });
    }

    /// Spawn a number field with the given label in-front of it.
    pub fn spawn_labeled_number_field<Generator: Component + NumberedEventGenerator>(
        &self,
        builder: &mut EntityCommands,
        size: Size,
        label: String,
        label_color: Color,
        number_field: NumberField<Generator>,
    ) {
        self.spawn_labeled(builder, size, label, label_color, |data, child_builder| {
            let mut number_bundle = data.get_text_bundle(
                number_field.current_value.to_string(),
                super::REGULAR_FONT_SIZE,
                Color::BLACK,
            );
            number_bundle.text.alignment.horizontal = HorizontalAlign::Right;
            child_builder
                .spawn_bundle(number_bundle)
                .insert(UiElement {
                    size: Size::new(size.width - 100.0, size.height - super::REGULAR_MARGIN),
                    selected_state: UiStateDetails {
                        accepts_state: true,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(number_field);
        });
    }
}

pub fn setup_menus(
    mut menu_data: ResMut<MenuState>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut events: EventWriter<ChangeViewTo>,
) {
    menu_data.button = asset_server.load("button.png");
    menu_data.font =
        asset_server.load("fonts/brass-mono-font-freeware-peter-fonseca/BrassMonoRegular-o2Yz.otf");
    menu_data.state_to_color.insert(0, Color::WHITE);
    menu_data.state_to_color.insert(1, Color::BLACK);

    // Here we will spawn the Panel that shows the buttons to change the tiling and
    // spawn the side panel that shows data about the rules.
    let mut tiling_button_group = commands.spawn();
    tiling_button_group.insert(AnchoredUi {
        x_percent: 0.5,
        y_percent: 1.0,
        width_grow: None,
        height_grow: None,
    });
    menu_data.build_button_group(
        &mut tiling_button_group,
        Color::WHITE,
        vec![
            (
                "Square".into(),
                Color::rgb(0.25, 0.5, 0.25),
                ChangeViewTo(TilingKind::Square),
            ),
            (
                "Hexagonal".into(),
                Color::rgb(0.5, 0.25, 0.25),
                ChangeViewTo(TilingKind::Hexagonal),
            ),
            (
                "Octagonal".into(),
                Color::rgb(0.25, 0.25, 0.5),
                ChangeViewTo(TilingKind::OctagonAndSquare),
            ),
            (
                "Equilateral Triangular".into(),
                Color::rgb(0.5, 0.25, 0.5),
                ChangeViewTo(TilingKind::EquilateralTriangular),
            ),
            (
                "Right Triangular".into(),
                Color::rgb(0.25, 0.5, 0.5),
                ChangeViewTo(TilingKind::RightTriangular),
            ),
        ],
        500.0,
        super::HEADER_HEIGHT,
        super::HEADER_FONT_SIZE,
        Color::WHITE,
        super::HEADER_MARGIN,
    );
    tiling_button_group.insert(Transform::from_translation(Vec3::new(0.0, 0.0, 10.0))); // Move it up.

    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(1.0, 1.0, 1.0, 0.5),
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ..Default::default()
        })
        .insert(UiElement {
            size: Size::new(300.0, 500.0),
            scroll_state: UiStateDetails {
                accepts_state: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(AnchoredUi {
            x_percent: 0.0,
            y_percent: 0.5,
            width_grow: None,
            height_grow: Some(1.0),
        })
        .insert(RulesContainer {})
        .insert(UiLinearScroll::default());

    let mut play_step = commands.spawn();
    play_step.insert(AnchoredUi {
        x_percent: 1.0,
        y_percent: 0.0,
        width_grow: None,
        height_grow: None,
    });
    menu_data.build_button_group(
        &mut play_step,
        Color::WHITE,
        vec![
            ("P".into(), Color::rgb(0.25, 0.5, 0.25), TogglePlay::Toggle),
            ("S".into(), Color::rgb(0.5, 0.25, 0.25), TogglePlay::Step),
        ],
        2.0 * super::HEADER_HEIGHT,
        super::HEADER_HEIGHT,
        super::HEADER_FONT_SIZE,
        Color::WHITE,
        super::HEADER_MARGIN,
    );
    play_step.insert(Transform::from_translation(Vec3::new(0.0, 0.0, 10.0))); // Move it up.

    events.send(ChangeViewTo(TilingKind::Square));
}
