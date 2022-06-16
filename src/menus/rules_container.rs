use bevy::{
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    math::Size,
    prelude::{Color, Commands, Component, Entity, EventReader, Query, Res, ResMut, With},
    sprite::SpriteBundle,
};

use crate::{
    simulation::{RuleUpdateTarget, SimulationState},
    ui::*,
};

use super::{events::*, MenuState};

#[derive(Component)]
pub struct RulesContainer {}

pub(super) fn change_rules_event(
    mut events: EventReader<ShowRulesFor>,
    rule_container_query: Query<(Entity, &UiElement), With<RulesContainer>>,
    mut commands: Commands,
    mut menu_data: ResMut<MenuState>,
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
                        super::HEADER_HEIGHT,
                        super::HEADER_FONT_SIZE,
                        Color::BLACK,
                        super::HEADER_MARGIN,
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
                    super::HEADER_HEIGHT,
                    super::HEADER_FONT_SIZE,
                    Color::BLACK,
                    super::HEADER_MARGIN,
                );

                let step_size = Size::new(element.size.width, super::REGULAR_HEIGHT_STEP);
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
                            super::HEADER_FONT_SIZE,
                            Color::BLACK,
                        ))
                        .insert(UiElement {
                            size: Size::new(element.size.width, super::HEADER_HEIGHT),
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
                                super::REGULAR_HEIGHT_STEP,
                                super::REGULAR_FONT_SIZE,
                                Color::GRAY,
                                super::REGULAR_MARGIN,
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
                            super::REGULAR_FONT_SIZE,
                            Color::BLACK,
                        ));
                    });
            });
        });
    }
}
