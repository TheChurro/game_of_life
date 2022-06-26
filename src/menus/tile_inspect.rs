use std::fmt::Debug;

use bevy::{
    hierarchy::{BuildChildren, DespawnRecursiveExt, Children},
    math::{IVec2, Vec3, Quat},
    prelude::{Color, Commands, Component, Entity, EventReader, Query, Res, With, Changed, KeyCode, ResMut, ParamSet, Visibility, Transform},
    utils::{HashSet, HashMap}, text::{Text, TextSection, TextStyle}, input::Input, pbr::MaterialMeshBundle,
};

use crate::{
    ui::{UiElement, UiLinearScroll, text_field::{TextEventGenerator, TextField}, AnchoredUi, InputState},
    visuals::{
        collapse::{CollapseEntry, CollapseState, CollapseEntryIndex},
        geom::{handles::GeometryHandleSet, GeomOrientation, GeometryStorage, WallProfileIndex, VerticalProfile, GeometryHandle, LayerProfileIndex, geom::DebugGeomDisplay},
    },
};

use super::{MenuState, REGULAR_FONT_SIZE, REGULAR_HEIGHT_STEP};

#[derive(Component, Clone, Debug)]
pub struct DebugTileEvent(pub IVec2);

#[derive(Component)]
pub struct DebugRoot {
    pub log_panel: Entity,
    pub input: Entity,
}

#[derive(Component)]
pub struct DebugState {
    pub debugging: bool,
    pub break_on: HashSet<CollapseEntryIndex>,
    pub breaking: bool,
    pub step: bool,
    pub display_options_for: HashMap<CollapseEntryIndex, HashMap<GeometryHandle, Vec<Entity>>>,
    pub remove_displays: Vec<HashMap<GeometryHandle, Vec<Entity>>>,
    pub wall_names: HashMap<WallProfileIndex, String>,
    pub layer_names: HashMap<LayerProfileIndex, String>,
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            debugging: false,
            break_on: Default::default(),
            breaking: false,
            step: false,
            display_options_for: Default::default(),
            remove_displays: Default::default(),
            wall_names: Default::default(),
            layer_names: Default::default()
        }
    }
}

pub fn update_debugger_panel(
    keyboard: Res<Input<KeyCode>>,
    input_state: Res<InputState>,
    mut debug_panel: Query<&mut AnchoredUi, With<DebugRoot>>,
    mut debug_state: ResMut<DebugState>,
) {
    if !input_state.has_selection() && keyboard.just_pressed(KeyCode::D) {
        debug_state.debugging = !debug_state.debugging;
        for mut anchor in debug_panel.iter_mut() {
            anchor.x_percent = if debug_state.debugging { 1.0 } else { 2.0 };
        }
    }
}

#[derive(Debug)]
enum ParseError {
    MissingTokens { num_tokens: usize, expected: usize },
    InvalidToken { position: usize, value: String, error: String },
    NoSuchCommand { command: String },
}

fn parse_tile_index(position: &mut usize, tokens: &Vec<&str>, is_last: bool) -> Result<CollapseEntryIndex, ParseError> {
    let mut tile_index = IVec2::ZERO;
    let mut height = 0;

    if *position + 1 >= tokens.len() {
        return Err(ParseError::MissingTokens { num_tokens: tokens.len(), expected: *position + 2 });
    }

    tile_index.x = tokens[*position].parse().map_err(|err| ParseError::InvalidToken {
        position: *position,
        value: tokens[*position].to_string(),
        error: format!("Parse X: {:?}", err),
    })?;

    tile_index.y = tokens[*position + 1].parse().map_err(|err| ParseError::InvalidToken {
        position: *position + 1,
        value: tokens[*position + 1].to_string(),
        error: format!("Parse Y: {:?}", err),
    })?;

    *position += 2;

    if *position < tokens.len() {
        match tokens[*position].parse() {
            Ok(parsed_height) => {
                height = parsed_height;
                *position += 1;
            }
            Err(err) => {
                if is_last {
                    return Err(ParseError::InvalidToken {
                        position: *position,
                        value: tokens[*position].to_string(),
                        error: format!("Parse Height: {:?}", err),
                    });
                }
            },
        }
    }

    Ok(CollapseEntryIndex::new(tile_index, height))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DebugTileOps {
    DisplayMeshes,
    PrintMeshes,
    PrintRestrictions,
}

enum DebugNameTarget {
    Wall,
    Layer,
}

enum DebugCommand {
    ToggleBreak { tile: CollapseEntryIndex },
    Continue,
    Step,
    DebugTile { tile: CollapseEntryIndex, debug_op: DebugTileOps },
    NameProfile { target: DebugNameTarget, index: usize, name: String },
    PrintMesh { mesh: GeometryHandle },
    Empty,
    Help,
}

fn parse_command(
    command: String,
) -> Result<DebugCommand, ParseError> {
    let tokens = command.split(" ").collect::<Vec<_>>();

    if tokens.len() == 0 {
        return Ok(DebugCommand::Empty);
    }

    let mut position = 1;
    match tokens[0] {
        "break" | "b" => {
            Ok(DebugCommand::ToggleBreak{
                tile: parse_tile_index(&mut position, &tokens, true)?
            })
        }
        "continue" | "c" => {
            Ok(DebugCommand::Continue)
        }
        "step" | "s" => {
            Ok(DebugCommand::Step)
        }
        "info" | "i" => {
            let tile = parse_tile_index(&mut position, &tokens, false)?;
            if position >= tokens.len() {
                return Err(ParseError::MissingTokens { num_tokens: tokens.len(), expected: position });
            }
            let debug_op = match tokens[position] {
                "display" | "d" => DebugTileOps::DisplayMeshes,
                "meshes" | "m" => DebugTileOps::PrintMeshes,
                "restrictions" | "r" => DebugTileOps::PrintRestrictions,
                _ => { 
                    return Err(ParseError::InvalidToken {
                        position,
                        value: tokens[position].to_string(),
                        error: "Invalid Command display(d), meshes(m) or restrictions(r)".to_string()
                    });
                }
            };
            Ok(DebugCommand::DebugTile{
                tile,
                debug_op,
            })
        }
        "name" | "n" => {
            if position >= tokens.len() {
                return Err(ParseError::MissingTokens { num_tokens: tokens.len(), expected: position });
            }
            let target = match tokens[position] {
                "wall" | "w" => DebugNameTarget::Wall,
                "layer" | "l" => DebugNameTarget::Layer,
                _ => return Err(ParseError::InvalidToken {
                    position,
                    value: tokens[position].to_string(),
                    error: format!("Expected wall(w) or layer(l)"),
                })
            };
            position += 1;
            if position >= tokens.len() {
                return Err(ParseError::MissingTokens { num_tokens: tokens.len(), expected: position });
            }
            let index = tokens[position].parse().map_err(|err| ParseError::InvalidToken {
                position,
                value: tokens[position].to_string(),
                error: format!("Failed to parse target index: {:?}", err),
            })?;
            position += 1;
            if position >= tokens.len() {
                return Err(ParseError::MissingTokens { num_tokens: tokens.len(), expected: position });
            }
            Ok(DebugCommand::NameProfile { target, index, name: tokens[position].to_string() })
        }
        "print" | "p" => {
            if position >= tokens.len() {
                return Err(ParseError::MissingTokens { num_tokens: tokens.len(), expected: position });
            }
            let handle_split = tokens[position].split("@").collect::<Vec<_>>();
            if handle_split.len() != 2 {
                return Err(ParseError::InvalidToken {
                    position,
                    value: tokens[position].to_string(),
                    error: "Expected index@orientation format".to_string()
                });
            }
            let index = handle_split[0].parse().map_err(|_| ParseError::InvalidToken {
                position,
                value: handle_split[0].to_string(),
                error: "Expected index@orientation but could not parse usize from index".to_string(),
            })?;
            let reverse = handle_split[1].chars().next() == Some('r');
            let rotation = handle_split[1][if reverse { 1 } else { 0 }..].parse().map_err(|_| ParseError::InvalidToken {
                position,
                value:handle_split[1].to_string(),
                error: "Expected index@orientation to be usize or r followed by a usize".to_string(),
            })?;

            let handle = GeometryHandle {
                index,
                orientation: if reverse {
                    GeomOrientation::Flipped { rotations: rotation }
                } else {
                    GeomOrientation::Standard { rotations: rotation }
                },
            };
            Ok(DebugCommand::PrintMesh { mesh: handle })
        }
        "help" | "h" => {
            Ok(DebugCommand::Help)
        }
        _ => Err(ParseError::NoSuchCommand { command: tokens[0].to_string() })
    }
}

pub fn inspect(
    mut events: EventReader<CommandEvent>,
    menu_data: Res<MenuState>,
    geom_data: Res<GeometryStorage>,
    mut debug_state: ResMut<DebugState>,
    collapse_state: Res<CollapseState>,
    collapse_query: Query<&CollapseEntry>,
    inspector_query: Query<&DebugRoot>,
    mut inspector_text_query: Query<&mut Children, With<UiLinearScroll>>,
    mut commands: Commands,
) {
    let mut new_text = Vec::new();
    for event in events.iter() {
        let command = match parse_command(event.0.clone()) {
            Ok(command) => command,
            Err(err) => {
                match err {
                    ParseError::MissingTokens { num_tokens, expected } => {
                        new_text.push(format!("Missing token! Have {} but expected {}", num_tokens, expected));
                    },
                    ParseError::InvalidToken { position, value, error } => {
                        new_text.push(format!("Invalid token {} (@{}): {}", value, position, error));
                    },
                    ParseError::NoSuchCommand { command } => {
                        new_text.push(format!("No such command {}", command));
                    },
                }
                continue;
            },
        };
        
        match command {
            DebugCommand::ToggleBreak { tile } => {
                if debug_state.break_on.contains(&tile) {
                    debug_state.break_on.remove(&tile);
                    new_text.push(format!("Breakpoint removed from {} at height {}", tile.index, tile.height));
                } else {
                    debug_state.break_on.insert(tile);
                    new_text.push(format!("Breakpoint set on {} at height {}", tile.index, tile.height));
                }
            },
            DebugCommand::Continue => { 
                debug_state.breaking = false;
                debug_state.step = true;
            },
            DebugCommand::Step => debug_state.step = true,
            DebugCommand::DebugTile { tile, debug_op } => {
                match debug_op {
                    DebugTileOps::DisplayMeshes => {
                        if !collapse_state.position_to_entry.contains_key(&tile) {
                            new_text.push(format!("Invalid tile {} at height {}", tile.index, tile.height));
                            continue;
                        }
                        if let Some(displays) = debug_state.display_options_for.remove(&tile) {
                            debug_state.remove_displays.push(displays);
                        } else {
                            debug_state.display_options_for.insert(tile, HashMap::new());
                        }
                    },
                    DebugTileOps::PrintMeshes | DebugTileOps::PrintRestrictions => {
                        let entity = match collapse_state.position_to_entry.get(&tile) {
                            Some(entity) => entity,
                            None => {
                                new_text.push(format!("Invalid tile {} at height {}", tile.index, tile.height));
                                continue;
                            },
                        };

                        let collapse_entry = match collapse_query.get(*entity) {
                            Ok(entry) => entry,
                            Err(_) => {
                                new_text.push(format!("Could not find entry for tile {} at height {}", tile.index, tile.height));
                                continue;
                            }
                        };

                        let collapse_entry: &CollapseEntry = collapse_entry;
                        if debug_op == DebugTileOps::PrintMeshes {
                            new_text.push(format!("Mesh for {}@{}: {}", tile.index, tile.height, GeometryHandle::pretty_string(collapse_entry.current_mesh)));
                            new_text.push(format!("Indicators: {} {}", VerticalProfile::create_label_string(collapse_entry.current_bottom_indicator), VerticalProfile::create_label_string(collapse_entry.current_top_indicator)));
                            new_text.push(format!("  Base: {}", collapse_entry.possible_geometry_entries_from_corner_data.data_string()));
                            let edge_restrictions = collapse_entry.compute_edge_restrictions(&geom_data);
                            for restriction in &edge_restrictions {
                                new_text.push(format!("  Edge Restriction: {}", restriction.data_string()));
                            }
                            let combined_restrictions = GeometryHandleSet::intersection(edge_restrictions.iter().chain([&collapse_entry.possible_geometry_entries_from_corner_data]));
                            new_text.push(format!("Combined Mesh: {}", combined_restrictions.data_string()));
                        } else {
                            new_text.push(format!("Edges restrictions for {} at height {}", tile.index, tile.height));
                            for restriction in &collapse_entry.edge_restrictions {
                                let mut walls = String::new();
                                for wall in WallProfileIndex::from_bits(restriction.restruction.unwrap_or(0)) {
                                    match debug_state.wall_names.get(&wall) {
                                        Some(name) => {
                                            walls.push_str(name);
                                            walls.push(' ');
                                        },
                                        None => {
                                            walls.push_str(&wall.index().to_string());
                                            walls.push(' ');
                                        },
                                    }
                                }
                                new_text.push(format!("  Edge {}: {}", restriction.edge, walls));
                            }
                        }
                    },
                }
            },
            DebugCommand::NameProfile { target, index, name } => {
                match target {
                    DebugNameTarget::Wall => {
                        debug_state.wall_names.insert(WallProfileIndex::new(index), name);
                    },
                    DebugNameTarget::Layer => {
                        debug_state.layer_names.insert(LayerProfileIndex::new(index), name);
                    },
                }
            },
            DebugCommand::PrintMesh { mesh } => {
                if mesh.index >= geom_data.profiles.len() {
                    new_text.push(format!("Index {} out of profile bounds!", mesh.index));
                }
                if !geom_data.profiles[mesh.index].orientations.contains(&mesh.orientation) {
                    new_text.push(format!("Orientation {:?} is not in mesh {}", mesh.orientation, mesh.index));
                }
                let mut data = String::new();
                let profile = &geom_data.profiles[mesh.index];
                for side in 0..profile.sides {
                    let wall = geom_data.get_wall(&profile, side, &mesh.orientation);
                    if let Some(name) = debug_state.wall_names.get(&wall) {
                        data.push_str(name);
                    } else {
                        data.push_str(&wall.index().to_string());
                    }
                    data.push(' ');
                }
                new_text.push(format!("Mesh {} has walls {}", mesh, data));
            },
            DebugCommand::Help => {
                new_text.push("Commands are".to_string());
                new_text.push("break(b) x y height".to_string());
                new_text.push("continue(c)        ".to_string());
                new_text.push("step(s)            ".to_string());
                new_text.push("info(i) x y height display(d)|meshes(m)|restrictions(r)".to_string());
                new_text.push("name(n) wall(w)|layer(l) index <value>".to_string());
                new_text.push("print(p) index@orientation".to_string());
            }
            DebugCommand::Empty => (),
        }
    }

    // Early out
    if new_text.is_empty() { return; }

    let root: &DebugRoot = inspector_query.single();
    match inspector_text_query.get_mut(root.log_panel) {
        Ok(scroll_children) => {
            let next_total_children = scroll_children.len() + new_text.len();
            if next_total_children >= 100 {
                for i in 0..next_total_children - 100 {
                    commands.entity(scroll_children[i]).despawn_recursive();
                }
            }
        },
        _ => ()
    };

    commands.entity(root.log_panel).with_children(|spawner| {
        for text in new_text {
            spawner.spawn_bundle(menu_data.get_ui_text_bundle(text, REGULAR_FONT_SIZE, 300.0, REGULAR_HEIGHT_STEP, Color::BLACK));
        }
    });
}

pub fn adjust_child_sizes(
    mut queries: ParamSet<(
        Query<(&DebugRoot, &UiElement), Changed<UiElement>>,
        Query<&mut UiElement>
    )>,
) {
    let mut updates = Vec::new();
    for (root, element) in queries.p0().iter() {
        updates.push((root.log_panel, root.input, element.size));
    }

    for (log_panel, input, containing_size) in updates {
        let input_size = if let Ok(element) = queries.p1().get(input) {
            element.size
        } else {
            continue;
        };

        if let Ok(mut element) = queries.p1().get_mut(log_panel) {
            element.size.height = containing_size.height - input_size.height;
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct CommandEventGenerator;

impl TextEventGenerator for CommandEventGenerator {
    type Event = CommandEvent;

    fn create_event(&self, value: String) -> Self::Event {
        CommandEvent(value)
    }
}

#[derive(Clone, Debug, Component)]
pub struct CommandEvent(pub String);

pub fn process_debug_inserts(
    mut query: Query<(&mut Text, &mut TextField<CommandEventGenerator>, &UiElement)>,
    mut events: EventReader<DebugTileEvent>,
) {
    for event in events.iter() {
        query.for_each_mut(|(mut text, mut text_field, element)| {
            if !element.selected_state.current {
                return;
            }
            text_field.current_value.push_str(&format!(" {} {}", event.0.x, event.0.y));

            if text.sections.len() == 0 {
                text.sections.push(TextSection {
                    value: text_field.current_value.clone(),
                    style: TextStyle {
                        font: Default::default(),
                        font_size: 14.0,
                        color: Color::BLACK,
                    },
                });
            } else {
                text.sections[0].value = text_field.current_value.clone();
            }
        });
    }
}

pub fn display_debug_options(
    mut debug_state: ResMut<DebugState>,
    geom_data: Res<GeometryStorage>,
    collapse_state: Res<CollapseState>,
    collapse_query: Query<&CollapseEntry>,
    mut commands: Commands
) {
    // Remove all displays that we need to get rid of
    for display in debug_state.remove_displays.drain(..) {
        for entities in display.values() {
            for entity in entities {
                commands.entity(*entity).despawn_recursive();
            }
        }
    }

    let is_debugging = debug_state.debugging;
    for (tile, displays) in &mut debug_state.display_options_for {
        let collapse_entity = if let Some(entity) = collapse_state.position_to_entry.get(tile) {
            entity
        } else {
            continue
        };

        let collapse_entry: &CollapseEntry = match collapse_query.get(*collapse_entity) {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let available_profiles = collapse_entry.compute_current_total_restriction(&geom_data);

        displays.drain_filter(|handle, entities| {
            if available_profiles.contains(*handle) {
                false
            } else {
                for entity in entities {
                    commands.entity(*entity).despawn_recursive();
                }
                true
            }
        });

        let pos = collapse_state.dual_tiling.get_tile_at_index(tile.index).position;

        for (y, handle) in available_profiles.into_iter().enumerate() {
            if !displays.contains_key(&handle) {
                let sides = geom_data.profiles[handle.index].sides;
                
                let base_transform = handle.orientation.get_transform(sides);
                let mut entities = Vec::new();
                let offset = Vec3::new(pos.x as f32, 1.5 + y as f32, pos.y as f32);
                entities.push(commands
                    .spawn_bundle(MaterialMeshBundle {
                        mesh: (&geom_data.mesh_handles[handle.index]).as_ref().map(|x| x.clone()).unwrap_or_default(),
                        material: geom_data.base_material.clone(),
                        transform: base_transform.with_translation(offset),
                        visibility: Visibility { is_visible: is_debugging },
                        ..Default::default()
                    })
                    .insert(DebugGeomDisplay).id());
                for side in 0..sides {
                    let angle = std::f32::consts::FRAC_PI_2 - std::f32::consts::TAU * side as f32 / sides as f32;
                    let transform = Transform::from_rotation(Quat::from_rotation_y(-std::f32::consts::TAU * (0.5 - side as f32 / sides as f32)))
                        .with_translation(offset + 0.5 * Vec3::new(angle.cos(), 0.0, angle.sin()));
                    let index = geom_data.get_wall(&geom_data.profiles[handle.index], side, &handle.orientation);
                    entities.push(commands.spawn_bundle(MaterialMeshBundle {
                        mesh: geom_data.profile_2d_meshes[index.index()].clone(),
                        material: geom_data.side_materials[index.index()].clone(),
                        visibility: Visibility { is_visible: is_debugging },
                        transform,
                        ..Default::default()
                    }).insert(DebugGeomDisplay).id());
                }
                displays.insert(handle, entities);
            }
        }
    }
}