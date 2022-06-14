use bevy::{
    ecs::system::EntityCommands,
    hierarchy::{BuildChildren, ChildBuilder, DespawnRecursiveExt},
    math::{IVec2, Size, Vec3},
    prelude::{
        AssetServer, Color, Commands, Component, Entity, EventReader, EventWriter, Handle, Image,
        Query, Res, ResMut, Transform, With, Assets,
    },
    sprite::{Sprite, SpriteBundle, ColorMaterial},
    text::{Font, HorizontalAlign, Text, Text2dBundle, TextAlignment, TextSection, TextStyle},
    transform::TransformBundle,
    utils::HashMap,
};

use crate::{
    generic_ui::{
        AnchoredUi, Button, LayoutDirection, NumberField, NumberedEventGenerator, UiElement,
        UiLinearScroll, UiStateDetails,
    },
    simulation::{RuleUpdateTarget, SimulationState},
    tiling::{TileShape, Tiling, TilingKind}, VisualsCache,
};

#[derive(Component, Clone, Copy)]
pub enum TogglePlay {
    Toggle,
    Step,
}

pub struct MenuData {
    pub button: Handle<Image>,
    pub font: Handle<Font>,
    pub active_shape: TileShape,
    pub active_state: u32,
    pub state_to_color: HashMap<u32, Color>,
}

impl Default for MenuData {
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

impl MenuData {
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

    pub fn build_button_group<Event: Component + Clone>(
        &self,
        builder: &mut EntityCommands,
        background: Color,
        mut data: Vec<(String, Color, Event)>,
        width: f32,
        height: f32,
        font_size: f32,
        font_color: Color,
        margin: f32,
    ) {
        builder
            .insert_bundle(SpriteBundle {
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
                            button_text
                                .spawn_bundle(self.get_text_bundle(text, font_size, font_color));
                        });
                }
            });
    }

    pub fn spawn_labeled(
        &self,
        builder: &mut EntityCommands,
        size: Size,
        label: String,
        label_color: Color,
        f: impl FnOnce(&Self, &mut ChildBuilder),
    ) {
        builder
            .insert_bundle(TransformBundle::default())
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
                    .spawn_bundle(self.get_text_bundle(label, REGULAR_FONT_SIZE, label_color))
                    .insert(UiElement {
                        size: Size::new(100.0, size.height - REGULAR_MARGIN),
                        ..Default::default()
                    });
                f(self, child_builder);
            });
    }

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
                REGULAR_FONT_SIZE,
                Color::BLACK,
            );
            number_bundle.text.alignment.horizontal = HorizontalAlign::Right;
            child_builder
                .spawn_bundle(number_bundle)
                .insert(UiElement {
                    size: Size::new(size.width - 100.0, size.height - REGULAR_MARGIN),
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

#[derive(Component, Clone)]
pub struct ChangeViewTo(pub TilingKind);

#[derive(Component)]
pub struct RulesContainer {}

#[derive(Component, Clone, Copy)]
pub struct ShowRulesFor {
    pub shape: TileShape,
    pub state: u32,
}

pub fn setup_menu_data(
    mut menu_data: ResMut<MenuData>,
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
        ],
        500.0,
        HEADER_HEIGHT,
        HEADER_FONT_SIZE,
        Color::WHITE,
        HEADER_MARGIN,
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
        2.0 * HEADER_HEIGHT,
        HEADER_HEIGHT,
        HEADER_FONT_SIZE,
        Color::WHITE,
        HEADER_MARGIN,
    );
    play_step.insert(Transform::from_translation(Vec3::new(0.0, 0.0, 10.0))); // Move it up.

    events.send(ChangeViewTo(TilingKind::Square));
}

pub fn change_view_to(
    mut events: EventReader<ChangeViewTo>,
    mut change_rules_view_events: EventWriter<ShowRulesFor>,
    mut sim_state: ResMut<SimulationState>,
) {
    for event in events.iter() {
        *sim_state = SimulationState::new(Tiling {
            kind: event.0,
            max_index: IVec2::new(50, 50),
        });

        change_rules_view_events.send(ShowRulesFor {
            shape: match sim_state.tiling.kind {
                TilingKind::Square => TileShape::Square,
                TilingKind::Hexagonal => TileShape::Hexagon,
                TilingKind::OctagonAndSquare => TileShape::Octagon,
            },
            state: 0u32,
        });
    }
}

pub fn toggle_play_event(
    mut events: EventReader<TogglePlay>,
    mut sim_state: ResMut<SimulationState>,
) {
    for event in events.iter() {
        match event {
            TogglePlay::Toggle => {
                sim_state.run_every = if sim_state.run_every == 0 { 10 } else { 0 }
            }
            TogglePlay::Step => {
                sim_state.step += 1;
            }
        }
    }
}

#[derive(Component)]
pub struct RuleUpdateEventGenerator {
    pub tile: TileShape,
    pub state: u32,
    pub rule_number: usize,
    pub target: RuleUpdateTarget,
}

impl NumberedEventGenerator for RuleUpdateEventGenerator {
    type Event = RuleUpdateEvent;

    fn create_event(&self, value: u32) -> Self::Event {
        RuleUpdateEvent::ModifyRule {
            shape: self.tile,
            state: self.state,
            rule_number: self.rule_number,
            value,
            target: self.target,
        }
    }
}

#[derive(Component, Clone, Copy)]
pub enum RuleUpdateEvent {
    ModifyRule {
        shape: TileShape,
        state: u32,
        rule_number: usize,
        value: u32,
        target: RuleUpdateTarget,
    },
    AddState {
        shape: TileShape,
    },
    AddRule {
        shape: TileShape,
        state: u32,
    },
    ShowRulesFor {
        shape: TileShape,
        state: u32,
    },
}

const HEADER_MARGIN: f32 = 20.0;
const HEADER_FONT_SIZE: f32 = 20.0;
const HEADER_HEIGHT: f32 = 50.0;
const REGULAR_MARGIN: f32 = 5.0;
const REGULAR_FONT_SIZE: f32 = 12.0;
const REGULAR_HEIGHT_STEP: f32 = 25.0;

pub fn change_rules_event(
    mut events: EventReader<ShowRulesFor>,
    rule_container_query: Query<(Entity, &UiElement), With<RulesContainer>>,
    mut commands: Commands,
    mut menu_data: ResMut<MenuData>,
    sim_state: Res<SimulationState>,
) {
    for event in events.iter() {
        // First update our menu data.
        menu_data.active_shape = event.shape;
        menu_data.active_state = event.state;

        // Next we are going to rebuild our rules container..
        rule_container_query.for_each(|(entity, element)| {
            let mut entity = commands.entity(entity);
            // Destroy any existing children. We will rebuild the ui from scratch
            entity.despawn_descendants();

            let valid_shapes = sim_state.get_shapes();
            let states = sim_state.clone_rules_for_shape(menu_data.active_shape);
            let num_states = states.len();

            entity.with_children(|child_builder| {
                // If we have multiple shapes allow the user to select a different
                // shape to display
                if valid_shapes.len() > 1 {
                    menu_data.build_button_group(
                        &mut child_builder.spawn(),
                        Color::WHITE,
                        valid_shapes
                            .iter()
                            .map(|shape| {
                                (
                                    shape.get_name(),
                                    if *shape == menu_data.active_shape {
                                        Color::GRAY
                                    } else {
                                        Color::WHITE
                                    },
                                    ShowRulesFor {
                                        shape: *shape,
                                        state: menu_data.active_state,
                                    },
                                )
                            })
                            .collect(),
                        element.size.width,
                        HEADER_HEIGHT,
                        HEADER_FONT_SIZE,
                        Color::BLACK,
                        HEADER_MARGIN,
                    );
                }

                // Next, if we have multiple states (which we always should) create a button group
                // for each of the states.
                menu_data.build_button_group(
                    &mut child_builder.spawn(),
                    Color::WHITE,
                    (0..num_states)
                        .map(|index| {
                            (
                                format!("{}", index),
                                if index as u32 == menu_data.active_state {
                                    Color::GRAY
                                } else {
                                    Color::WHITE
                                },
                                RuleUpdateEvent::ShowRulesFor {
                                    shape: menu_data.active_shape,
                                    state: index as u32,
                                },
                            )
                        })
                        .chain([(
                            "+".to_string(),
                            Color::WHITE,
                            RuleUpdateEvent::AddState {
                                shape: menu_data.active_shape,
                            },
                        )])
                        .collect(),
                    element.size.width,
                    HEADER_HEIGHT,
                    HEADER_FONT_SIZE,
                    Color::BLACK,
                    HEADER_MARGIN,
                );

                let step_size = Size::new(element.size.width, REGULAR_HEIGHT_STEP);
                let num_rules = states.len() as u32;
                let rule_set = &states[menu_data.active_state as usize];

                menu_data.spawn_labeled_number_field(
                    &mut child_builder.spawn(),
                    step_size,
                    "Default:".into(),
                    Color::BLACK,
                    NumberField {
                        event_generator: RuleUpdateEventGenerator {
                            tile: menu_data.active_shape,
                            state: menu_data.active_state,
                            rule_number: 0,
                            target: RuleUpdateTarget::DefaultValue,
                        },
                        current_value: rule_set.default_state,
                        min_value: 0,
                        max_value: num_rules - 1,
                    },
                );

                for (i, rule) in rule_set.rules.iter().enumerate() {
                    child_builder
                        .spawn_bundle(menu_data.get_text_bundle(
                            format!("Rule {}", i),
                            HEADER_FONT_SIZE,
                            Color::BLACK,
                        ))
                        .insert(UiElement {
                            size: Size::new(element.size.width, HEADER_HEIGHT),
                            ..Default::default()
                        });
                    menu_data.spawn_labeled(
                        &mut child_builder.spawn(),
                        step_size,
                        "Count State:".into(),
                        Color::BLACK,
                        |data, count_builder| {
                            data.build_button_group(
                                &mut count_builder.spawn(),
                                Color::WHITE,
                                (0..sim_state.num_states)
                                    .map(|index| {
                                        (
                                            format!("{}", index),
                                            if rule
                                                .neighbor_states_to_count
                                                .contains(&(index as u32))
                                            {
                                                Color::BLACK
                                            } else {
                                                Color::WHITE
                                            },
                                            RuleUpdateEvent::ModifyRule {
                                                shape: menu_data.active_shape,
                                                state: menu_data.active_state,
                                                rule_number: i,
                                                value: index as u32,
                                                target: RuleUpdateTarget::ToggleCount,
                                            },
                                        )
                                    })
                                    .collect(),
                                element.size.width - 100.0,
                                REGULAR_HEIGHT_STEP,
                                REGULAR_FONT_SIZE,
                                Color::GRAY,
                                REGULAR_MARGIN,
                            );
                        },
                    );

                    menu_data.spawn_labeled_number_field(
                        &mut child_builder.spawn(),
                        step_size,
                        "Min:".into(),
                        Color::BLACK,
                        NumberField {
                            event_generator: RuleUpdateEventGenerator {
                                tile: menu_data.active_shape,
                                state: menu_data.active_state,
                                rule_number: i,
                                target: RuleUpdateTarget::MinValue,
                            },
                            current_value: rule.min,
                            max_value: 8,
                            min_value: 0,
                        },
                    );

                    menu_data.spawn_labeled_number_field(
                        &mut child_builder.spawn(),
                        step_size,
                        "Max:".into(),
                        Color::BLACK,
                        NumberField {
                            event_generator: RuleUpdateEventGenerator {
                                tile: menu_data.active_shape,
                                state: menu_data.active_state,
                                rule_number: i,
                                target: RuleUpdateTarget::MaxValue,
                            },
                            current_value: rule.max,
                            max_value: 8,
                            min_value: 0,
                        },
                    );

                    menu_data.spawn_labeled_number_field(
                        &mut child_builder.spawn(),
                        step_size,
                        "Output:".into(),
                        Color::BLACK,
                        NumberField {
                            event_generator: RuleUpdateEventGenerator {
                                tile: menu_data.active_shape,
                                state: menu_data.active_state,
                                rule_number: i,
                                target: RuleUpdateTarget::ResultValue,
                            },
                            current_value: rule.output,
                            max_value: num_rules - 1,
                            min_value: 0,
                        },
                    );
                }

                child_builder
                    .spawn_bundle(SpriteBundle {
                        texture: menu_data.button.clone(),
                        ..Default::default()
                    })
                    .insert(UiElement {
                        size: step_size,
                        click_state: UiStateDetails {
                            accepts_state: true,
                            ..UiStateDetails::default()
                        },
                        ..Default::default()
                    })
                    .insert(Button::new(
                        menu_data.button.clone(),
                        RuleUpdateEvent::AddRule {
                            shape: menu_data.active_shape,
                            state: menu_data.active_state,
                        },
                    ))
                    .with_children(|child_builder| {
                        child_builder.spawn_bundle(menu_data.get_text_bundle(
                            "Add Rule".to_string(),
                            REGULAR_FONT_SIZE,
                            Color::BLACK,
                        ));
                    });
            });
        });
    }
}

pub fn on_rule_update(
    mut events: EventReader<RuleUpdateEvent>,
    mut sim_state: ResMut<SimulationState>,
    mut out_events: EventWriter<ShowRulesFor>,
    mut menu_state: ResMut<MenuData>,
    mut vis_cache: ResMut<VisualsCache>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for event in events.iter() {
        let update_view;
        let mut show_rule_event = ShowRulesFor {
            shape: menu_state.active_shape,
            state: menu_state.active_state,
        };
        match *event {
            RuleUpdateEvent::ModifyRule {
                shape,
                state,
                rule_number,
                value,
                target,
            } => {
                update_view = target == RuleUpdateTarget::ToggleCount;
                sim_state.set_rule_value(shape, state, rule_number, value, target);
            }
            RuleUpdateEvent::AddState { shape: tile } => {
                sim_state.add_state(tile);
                let new_state = sim_state.num_states as u32 - 1;
                if !menu_state.state_to_color.contains_key(&new_state) {
                    let color = Color::hsl(((new_state * 37) % 360) as f32, 1.0, 0.75);
                    menu_state.state_to_color.insert(new_state, color);
                    let image = vis_cache.outline_image.clone();
                    vis_cache.states.insert(
                        new_state,
                        materials.add(ColorMaterial {
                            color,
                            texture: Some(image),
                        }),
                    );
                }
                update_view = true;
            }
            RuleUpdateEvent::AddRule { shape: tile, state } => {
                sim_state.add_rule(tile, state);
                update_view = true;
            }
            RuleUpdateEvent::ShowRulesFor { shape, state } => {
                show_rule_event = ShowRulesFor { shape, state };
                update_view = true;
            }
        }

        if update_view {
            out_events.send(show_rule_event);
        }
    }
}
