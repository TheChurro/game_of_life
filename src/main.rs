use bevy::{
    hierarchy::{BuildChildren, Children, Parent},
    input::{
        mouse::{MouseMotion, MouseWheel},
        Input,
    },
    math::{vec2, IVec2, Vec2, Vec3, Vec3Swizzles},
    prelude::{
        App, AssetServer, Assets, Changed, Color, Commands, Component, CoreStage, Entity,
        EventReader, Handle, Mesh, MouseButton, OrthographicCameraBundle,
        ParallelSystemDescriptorCoercion, Query, Res, ResMut, Transform, With, Without, Image,
    },
    render::{mesh::{Indices, PrimitiveTopology}},
    sprite::{ColorMaterial, MaterialMesh2dBundle, Mesh2dHandle},
    text::{Font, Text, Text2dBundle, TextAlignment, TextSection, TextStyle},
    utils::HashMap,
    window::Windows,
    DefaultPlugins,
};
use generic_ui::{number_field_handler, UiElement};
use menu_ui::{
    on_rule_update, toggle_play_event, RuleUpdateEvent, RuleUpdateEventGenerator, TogglePlay,
};
use simulation::SimulationState;
use tiling::{TileShape, Tiling, TilingKind};

use crate::{
    generic_ui::{
        button_handler, linear_scroll_children_changed, linear_scroll_handler, position_on_added,
        position_on_window_changed, update_sprite_to_match_layout,
    },
    menu_ui::{
        change_rules_event, change_view_to, setup_menu_data, ChangeViewTo, MenuData, ShowRulesFor,
    },
};

extern crate bevy;

mod generic_ui;
mod menu_ui;
mod simulation;
mod tiling;

#[derive(Component)]
struct VisualState {
    cur_offset: Vec2,
    visual_grid_count: IVec2,
    scale: f32,
    min_scale: f32,
    max_scale: f32,
    add_debug: bool,
}

#[derive(Component)]
pub struct VisualsCache {
    meshes: HashMap<TileShape, Mesh2dHandle>,
    states: HashMap<u32, Handle<ColorMaterial>>,
    outline_image: Handle<Image>,
    font: Handle<Font>,
}

#[derive(Component)]
struct TileState {
    offset_from_center: IVec2,
    computed_index: IVec2,
    current_state: u32,
    previous_shape: TileShape,
    alive_count: u32,
    dead_count: u32,
    next: u32,
}

struct InputState {
    is_mouse_down: bool,
    mouse_moved: bool,
    ui_element_clicked: Option<Entity>,
    ui_element_clicked_buffered: Option<Entity>,
    ui_element_selected: Option<Entity>,
    ui_element_selected_buffered: Option<Entity>,
    ui_element_scrolled: Option<Entity>,
    last_mouse_position: Vec2,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            is_mouse_down: false,
            mouse_moved: false,
            ui_element_clicked: None,
            ui_element_clicked_buffered: None,
            ui_element_selected: None,
            ui_element_selected_buffered: None,
            ui_element_scrolled: None,
            last_mouse_position: Vec2::ZERO,
        }
    }
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut visuals_cache: ResMut<VisualsCache>,
    sim_state: Res<SimulationState>,
    vis_state: Res<VisualState>,
    menu_state: Res<MenuData>,
) {
    for shape in [TileShape::Square, TileShape::Hexagon, TileShape::Octagon] {
        let mut verticies = vec![[0.0, 0.0, 0.0]];
        let mut normals = vec![[0.0, 0.0, 1.0]];
        let mut uvs = vec![[0.5, 0.5]];
        let mut indicies = vec![];
        let num_sides = shape.get_side_count();
        let angle = std::f32::consts::TAU / num_sides as f32;
        for i in 0..num_sides {
            let cur_angle = angle * (0.5 + i as f32);
            let radius = shape.get_radius();
            verticies.push([radius * cur_angle.cos(), radius * cur_angle.sin(), 0.0]);
            uvs.push([i as f32 / (num_sides - 1) as f32, 0.0]);
            normals.push([0.0, 0.0, 1.0]);
            indicies.extend_from_slice(&[0, 1 + i, 1 + ((i + 1) % num_sides)]);
        }
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verticies);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_indices(Some(Indices::U32(indicies)));

        let handle = meshes.add(mesh);
        visuals_cache.meshes.insert(shape, handle.into());
    }

    let outline_img = asset_server.load("Outline.png");

    visuals_cache.outline_image = outline_img.clone();
    visuals_cache.states.insert(
        0,
        materials.add(ColorMaterial {
            color: menu_state
                .state_to_color
                .get(&0u32)
                .cloned()
                .unwrap_or(Color::GRAY),
            texture: Some(outline_img.clone()),
        }),
    );
    visuals_cache.states.insert(
        1,
        materials.add(ColorMaterial {
            color: menu_state
                .state_to_color
                .get(&1u32)
                .cloned()
                .unwrap_or(Color::GRAY),
            texture: Some(outline_img.clone()),
        }),
    );

    visuals_cache.font = asset_server
        .load("fonts/brass-mono-font-freeware-peter-fonseca/BrassMonoCozyRegular-g146.otf");

    let default_color = visuals_cache
        .states
        .get(&0)
        .expect("Failed to get material just created!")
        .clone();

    let half_size = Vec2::ZERO; //sim_state.tiling.size() / 2.0;
    for x in -vis_state.visual_grid_count.x / 2..(vis_state.visual_grid_count.x + 1) / 2 {
        for y in -vis_state.visual_grid_count.y / 2..(vis_state.visual_grid_count.y + 1) / 2 {
            let tile = sim_state.tiling.get_tile_at_index(IVec2::new(x, y));

            let mut entity = commands.spawn_bundle(MaterialMesh2dBundle {
                mesh: visuals_cache
                    .meshes
                    .get(&tile.shape)
                    .expect("Failed to get mesh we just inserted!")
                    .clone(),
                material: default_color.clone(),
                transform: Transform::from_translation((tile.position - half_size).extend(0.0)),
                ..Default::default()
            });
            entity.insert(TileState {
                offset_from_center: IVec2::new(x, y),
                computed_index: sim_state.tiling.adjust_index(IVec2::new(x, y)),
                current_state: if x == 25 { 1 } else { 0 },
                previous_shape: tile.shape,
                alive_count: 0,
                dead_count: sim_state.tiling.get_neighbors(IVec2::new(x, y)).len() as u32,
                next: 0,
            });
            if vis_state.add_debug {
                entity.with_children(|child_builder| {
                    child_builder.spawn_bundle(Text2dBundle {
                        text: Text {
                            sections: Vec::new(),
                            alignment: TextAlignment {
                                vertical: bevy::text::VerticalAlign::Center,
                                horizontal: bevy::text::HorizontalAlign::Center,
                            },
                        },
                        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                        ..Default::default()
                    });
                });
            }
        }
    }

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn update_tile(
    mut tile_query: Query<(&mut Transform, &mut TileState)>,
    vis_state: Res<VisualState>,
    sim_state: Res<SimulationState>,
) {
    let central_tile = sim_state.tiling.get_tile_containing(vis_state.cur_offset);
    let mut offset = central_tile.position - vis_state.cur_offset;
    // This is super hacky way to make sure we wrap smoothly but whatever...
    let tiling_size = sim_state.tiling.size();
    if offset.x > tiling_size.x / 2.0 {
        offset.x -= tiling_size.x;
    } else if offset.x < tiling_size.x / -2.0 {
        offset.x += tiling_size.x;
    }
    if offset.y > tiling_size.y / 2.0 {
        offset.y -= tiling_size.y;
    } else if offset.y < tiling_size.y / -2.0 {
        offset.y += tiling_size.y;
    }
    tile_query.for_each_mut(|(mut transform, mut state)| {
        let new_index = sim_state
            .tiling
            .adjust_index(central_tile.index + state.offset_from_center);
        if state.computed_index != new_index {
            state.computed_index = new_index;
        }
        let new_state = sim_state.get_at(new_index);
        if new_state != state.current_state {
            state.current_state = new_state;
        }
        let shape = sim_state.tiling.get_tile_at_index(new_index).shape;
        if shape != state.previous_shape {
            state.previous_shape = shape;
        }
        let alive_neighbors = sim_state.get_neighbor_count(new_index, 1);
        if alive_neighbors != state.alive_count {
            state.alive_count = alive_neighbors;
        }
        let dead_neighbors = sim_state.get_neighbor_count(new_index, 0);
        if dead_neighbors != state.dead_count {
            state.dead_count = dead_neighbors;
        }
        let pending = sim_state.get_pending(new_index);
        if pending != state.next {
            state.next = pending;
        }
        transform.translation = vis_state.scale
            * (offset
                + sim_state.tiling.compute_offset_between_indicies(
                    central_tile.index,
                    central_tile.index + state.offset_from_center,
                ))
            .extend(0.0);
        transform.scale = vis_state.scale * Vec3::ONE;
    });
}

fn update_tile_visual(
    mut tile_query: Query<
        (
            &mut Mesh2dHandle,
            &mut Handle<ColorMaterial>,
            &TileState,
            Option<&Children>,
        ),
        Changed<TileState>,
    >,
    mut text_query: Query<(&mut Transform, &mut Text)>,
    visuals_cache: Res<VisualsCache>,
    vis_state: Res<VisualState>,
    sim_state: Res<SimulationState>,
) {
    tile_query.for_each_mut(|(mut mesh, mut material, state, children)| {
        *mesh = visuals_cache
            .meshes
            .get(
                &sim_state
                    .tiling
                    .get_tile_at_index(state.computed_index)
                    .shape,
            )
            .expect("Failed to get mesh that should be registered!")
            .clone();
        *material = visuals_cache
            .states
            .get(&state.current_state)
            .expect("Failed to get material that should be registered!")
            .clone();
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok((mut transform, mut text)) = text_query.get_mut(*child) {
                    transform.scale = Vec3::ONE / vis_state.scale;
                    text.sections.clear();
                    text.sections.push(TextSection {
                        value: format!(
                            "D{}A{}N{}",
                            state.dead_count, state.alive_count, state.next
                        ),
                        style: TextStyle {
                            font: visuals_cache.font.clone(),
                            font_size: 12.0,
                            color: Color::RED,
                        },
                    });
                }
            }
        }
    });
}

const SCROLL_SENSITIVITY: f32 = 0.5;

fn find_event_targets(
    entity: Entity,
    valid_hover: bool,
    mut hover_position: Vec2,
    mut clear_hover_position: Vec2,
    ui_element_query: &mut Query<(&Transform, &mut UiElement, Option<&Children>)>,
) -> (bool, Option<Entity>, Option<Entity>, Option<Entity>) {
    let mut is_hovered = false;
    let mut is_hover_cleared = false;

    let mut click_target = None;
    let mut scroll_target = None;
    let mut select_target = None;

    let children =
        if let Ok((transform, mut element, maybe_children)) = ui_element_query.get_mut(entity) {
            // Check to see if we are hovered (or we were hovered last frame).
            hover_position -= transform.translation.xy();
            clear_hover_position -= transform.translation.xy();

            is_hovered = valid_hover
                && hover_position.x.abs() <= element.size.width / 2.0
                && hover_position.y.abs() <= element.size.height / 2.0;
            is_hover_cleared = clear_hover_position.x.abs() <= element.size.width / 2.0
                && clear_hover_position.y.abs() <= element.size.height / 2.0;

            // If this element can be hovered and hover has changed, update that state.
            if element.hover_state.accepts_state {
                if element.hover_state.current != is_hovered {
                    element.hover_state.previous = !is_hovered;
                    element.hover_state.current = is_hovered;
                }
            }

            if is_hovered {
                click_target = if element.click_state.accepts_state {
                    Some(entity)
                } else {
                    None
                };
                scroll_target = if element.scroll_state.accepts_state {
                    Some(entity)
                } else {
                    None
                };
                select_target = if element.selected_state.accepts_state {
                    Some(entity)
                } else {
                    None
                };
            }

            if let Some(children) = maybe_children {
                children.iter().cloned().collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

    // If we are hovered, or if last tick we should have been hovered,
    // the pass the hover onto our children who will do the same checks.
    if is_hovered || is_hover_cleared {
        for child in children.iter().rev() {
            let (_, child_click_target, child_scroll_target, child_select_target) =
                find_event_targets(
                    *child,
                    is_hovered,
                    hover_position,
                    clear_hover_position,
                    ui_element_query,
                );
            click_target = child_click_target.or(click_target);
            scroll_target = child_scroll_target.or(scroll_target);
            select_target = child_select_target.or(select_target);
        }
    }

    (is_hovered, click_target, scroll_target, select_target)
}

fn input_system(
    mouse_input: Res<Input<MouseButton>>,
    mut mouse_state: ResMut<InputState>,
    mut mouse_movements: EventReader<MouseMotion>,
    mut mouse_wheel_movements: EventReader<MouseWheel>,
    mut vis_state: ResMut<VisualState>,
    mut sim_state: ResMut<SimulationState>,
    windows: Res<Windows>,
    ui_roots_query: Query<Entity, (With<UiElement>, Without<Parent>)>,
    mut ui_element_query: Query<(&Transform, &mut UiElement, Option<&Children>)>,
) {
    let mut scroll = Vec2::ZERO;
    for motion in mouse_wheel_movements.iter() {
        scroll += Vec2::new(motion.x, motion.y) * SCROLL_SENSITIVITY;
    }

    // Adjust the scroll for our last scrolled entity.
    if let Some(entity) = mouse_state.ui_element_scrolled {
        if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
            element.scroll_state.previous = element.scroll_state.current;
            element.scroll_state.current = Vec2::ZERO;
        }
    }
    mouse_state.ui_element_scrolled = None;

    // Adjust the selected element state for the last selected entity.
    if mouse_state.ui_element_selected != mouse_state.ui_element_selected_buffered {
        if let Some(entity) = mouse_state.ui_element_selected_buffered {
            if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                element.selected_state.previous = element.selected_state.current;
                element.selected_state.current = false;
            }
        }
    }
    mouse_state.ui_element_selected_buffered = None;

    let mut clear_select = false;
    if let Some(entity) = mouse_state.ui_element_selected {
        clear_select = mouse_input.just_pressed(MouseButton::Left);
        if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
            element.selected_state.previous = element.selected_state.current;
            element.selected_state.current = !clear_select;
        }
    }
    if clear_select {
        mouse_state.ui_element_selected_buffered = mouse_state.ui_element_selected;
        mouse_state.ui_element_selected = None;
    }

    // Adjust our click states
    if mouse_state.ui_element_clicked != mouse_state.ui_element_clicked_buffered {
        if let Some(entity) = mouse_state.ui_element_clicked_buffered {
            if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                element.click_state.previous = element.click_state.current;
                element.click_state.current = false;
            }
        }
    }
    mouse_state.ui_element_clicked_buffered = None;

    let mut clear_click = false;
    if let Some(entity) = mouse_state.ui_element_clicked {
        clear_click = !mouse_input.pressed(MouseButton::Left);
        if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
            element.click_state.previous = element.click_state.current;
            if clear_click {
                element.click_state.current = false;
            }
        }
    }
    if clear_click {
        mouse_state.ui_element_clicked_buffered = mouse_state.ui_element_clicked;
        mouse_state.ui_element_clicked = None;
    }

    let mut over_ui = false;
    // If we have a mouse position, we are going to go issue hovers, clicks, selects and scrolls
    if let Some(mouse_position) = windows
        .get_primary()
        .and_then(|window| window.cursor_position())
    {
        let mouse_position =
            mouse_position - Vec2::new(windows.primary().width(), windows.primary().height()) * 0.5;
        let mut click_target = None;
        let mut scroll_target = None;
        let mut select_target = None;

        for root in ui_roots_query.iter() {
            let (is_hovered, root_click_target, root_scroll_target, root_select_target) =
                find_event_targets(
                    root,
                    true,
                    mouse_position,
                    mouse_state.last_mouse_position,
                    &mut ui_element_query,
                );
            click_target = root_click_target.or(click_target);
            scroll_target = root_scroll_target.or(scroll_target);
            select_target = root_select_target.or(select_target);

            over_ui |= is_hovered;
        }

        if mouse_input.just_pressed(MouseButton::Left) {
            if let Some(entity) = click_target {
                if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                    element.click_state.previous = element.click_state.current;
                    element.click_state.current = true;
                }
                mouse_state.ui_element_clicked = Some(entity);
            } else if let Some(entity) = select_target {
                if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                    element.selected_state.previous = element.selected_state.current;
                    element.selected_state.current = true;
                }
                mouse_state.ui_element_selected = Some(entity);
            }
        } else if !mouse_input.pressed(MouseButton::Left) {
            if let Some(entity) = scroll_target {
                if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                    element.scroll_state.previous = element.scroll_state.current;
                    element.scroll_state.current = scroll;
                }
                mouse_state.ui_element_scrolled = Some(entity);
            }
        }

        mouse_state.last_mouse_position = mouse_position;
    };

    if over_ui {
        return;
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        mouse_state.is_mouse_down = true;
        mouse_state.mouse_moved = false;
    }

    for motion in mouse_movements.iter() {
        if motion.delta != Vec2::ZERO && mouse_state.is_mouse_down {
            mouse_state.mouse_moved = true;
            vis_state.cur_offset = sim_state.tiling.adjust_position(
                motion.delta * vec2(-1.0, 1.0) / vis_state.scale + vis_state.cur_offset,
            );
        }
    }

    vis_state.scale = (vis_state.scale + scroll.y * SCROLL_SENSITIVITY)
        .max(vis_state.min_scale)
        .min(vis_state.max_scale);

    if mouse_input.just_released(MouseButton::Left) {
        mouse_state.is_mouse_down = false;
        if !mouse_state.mouse_moved {
            let primary_window = windows.primary();
            if let Some(position) = primary_window.cursor_position() {
                let position =
                    position - Vec2::new(primary_window.width(), primary_window.height()) / 2.0;
                let adjusted_position = position / vis_state.scale + vis_state.cur_offset;
                let index = sim_state.tiling.get_index_for_position(adjusted_position);
                let target_state = (sim_state.get_at(index) + 1) % sim_state.num_states as u32;
                sim_state.set_at(index, target_state);
            }
        }
    }
}

fn process_simulation(mut sim_state: ResMut<SimulationState>) {
    sim_state.process();
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.insert_resource(VisualsCache {
            meshes: Default::default(),
            states: Default::default(),
            outline_image: Default::default(),
            font: Handle::default(),
        })
        .insert_resource(SimulationState::new(Tiling {
            kind: TilingKind::Hexagonal,
            max_index: IVec2::new(50, 50),
        }))
        .insert_resource(VisualState {
            cur_offset: Vec2::ZERO,
            visual_grid_count: IVec2::new(25, 25),
            scale: 50.0,
            min_scale: 25.0,
            max_scale: 100.0,
            add_debug: false,
        })
        .insert_resource(MenuData {
            active_shape: TileShape::Hexagon,
            ..Default::default()
        })
        .insert_resource(InputState::default())
        .add_event::<ChangeViewTo>()
        .add_event::<ShowRulesFor>()
        .add_event::<TogglePlay>()
        .add_event::<RuleUpdateEvent>()
        .add_startup_system(setup_menu_data)
        .add_startup_system(setup_world.after(setup_menu_data))
        .add_system(change_view_to)
        .add_system(change_rules_event)
        .add_system_to_stage(CoreStage::PreUpdate, input_system)
        .add_system(update_tile)
        .add_system(update_tile_visual.after(update_tile))
        .add_system(update_sprite_to_match_layout)
        .add_system(linear_scroll_children_changed)
        .add_system(linear_scroll_handler)
        .add_system(change_view_to)
        .add_system(button_handler::<ChangeViewTo>)
        .add_system(button_handler::<ShowRulesFor>)
        .add_system(button_handler::<TogglePlay>)
        .add_system(button_handler::<RuleUpdateEvent>)
        .add_system(number_field_handler::<RuleUpdateEventGenerator>)
        .add_system(position_on_added)
        .add_system(position_on_window_changed)
        .add_system(process_simulation)
        .add_system(toggle_play_event)
        .add_system(on_rule_update)
        .run()
}
