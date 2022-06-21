use bevy::{
    math::{IVec2, Vec2},
    prelude::{Assets, Color, Component, EventReader, EventWriter, ResMut},
    sprite::ColorMaterial,
};

use crate::{
    simulation::{RuleUpdateTarget, SimulationState},
    tiling::{EquilateralDirection, RightTriangleRotation, TileShape, Tiling, TilingKind},
    ui::NumberedEventGenerator,
    visuals::collapse::SimulationStateChanged,
    VisualsCache,
};

use super::MenuState;

#[derive(Component, Clone, Copy)]
pub enum TogglePlay {
    Toggle,
    Step,
}

#[derive(Component, Clone)]
pub struct ChangeViewTo(pub TilingKind);

#[derive(Component, Clone, Copy)]
pub struct ShowRulesFor {
    pub shape: TileShape,
    pub state: u32,
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

pub(super) fn change_view_to(
    mut events: EventReader<ChangeViewTo>,
    mut change_rules_view_events: EventWriter<ShowRulesFor>,
    mut out_vis_events: EventWriter<SimulationStateChanged>,
    mut sim_state: ResMut<SimulationState>,
) {
    for event in events.iter() {
        let grid_size = if event.0 == TilingKind::Square {
            52
        } else {
            52
        };
        *sim_state = SimulationState::new(Tiling {
            kind: event.0,
            max_index: IVec2::new(grid_size, grid_size),
            offset: Vec2::ZERO,
        });

        change_rules_view_events.send(ShowRulesFor {
            shape: match sim_state.tiling.kind {
                TilingKind::Square => TileShape::Square,
                TilingKind::Hexagonal => TileShape::Hexagon,
                TilingKind::OctagonAndSquare => TileShape::Octagon,
                TilingKind::EquilateralTriangular => {
                    TileShape::EquilateralTriangle(EquilateralDirection::Up)
                }
                TilingKind::RightTriangular => {
                    TileShape::RightTriangle(RightTriangleRotation::Zero)
                }
            },
            state: 0u32,
        });

        out_vis_events.send(SimulationStateChanged::NewTiling);
    }
}

pub(super) fn toggle_play_event(
    mut events: EventReader<TogglePlay>,
    mut sim_state: ResMut<SimulationState>,
) {
    for event in events.iter() {
        match event {
            TogglePlay::Toggle => {
                sim_state.run_every = if sim_state.run_every == 0 { 5 } else { 0 }
            }
            TogglePlay::Step => {
                sim_state.step += 1;
            }
        }
    }
}

pub(super) fn on_rule_update(
    mut events: EventReader<RuleUpdateEvent>,
    mut sim_state: ResMut<SimulationState>,
    mut out_events: EventWriter<ShowRulesFor>,
    mut out_vis_events: EventWriter<SimulationStateChanged>,
    mut menu_state: ResMut<MenuState>,
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
                    out_vis_events.send(SimulationStateChanged::NewTiling);
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
