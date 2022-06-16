use bevy::{
    hierarchy::{BuildChildren, Children, Parent},
    input::{
        mouse::{MouseMotion, MouseWheel},
        Input,
    },
    math::{IVec2, Vec2, Vec3},
    prelude::{
        App, AssetServer, Assets, Changed, Color, Commands, Component, CoreStage, Entity,
        EventReader, Handle, Image, Mesh, MouseButton, OrthographicCameraBundle,
        ParallelSystemDescriptorCoercion, Query, Res, ResMut, Transform, With, Without,
    },
    render::mesh::{Indices, PrimitiveTopology},
    sprite::{ColorMaterial, MaterialMesh2dBundle, Mesh2dHandle},
    text::{Font, Text, Text2dBundle, TextAlignment, TextSection, TextStyle},
    utils::HashMap,
    window::Windows,
    DefaultPlugins,
};

use menus::MenuState;
use simulation::SimulationState;
use tiling::{TileShape, Tiling, TilingKind, EquilateralDirection, RightTriangleRotation, OCTAGON_SQUARE_DIFFERENCE_OF_CENTER};

extern crate bevy;

mod menus;
mod simulation;
mod tiling;
mod ui;
mod visuals;

#[derive(Component)]
struct VisualState {
    mouse_down: bool,
    mouse_moved: bool,

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

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut visuals_cache: ResMut<VisualsCache>,
    sim_state: Res<SimulationState>,
    vis_state: Res<VisualState>,
    menu_state: Res<MenuState>,
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

    // Now we handle the angles for the two different kinds of equilateral triangles
    for direction in [EquilateralDirection::Down, EquilateralDirection::Up] {
        let shape = TileShape::EquilateralTriangle(direction);
        let mut verticies = vec![[0.0, 0.0, 0.0]];
        let mut normals = vec![[0.0, 0.0, 1.0]];
        let mut uvs = vec![[0.5, 0.5]];
        let mut indicies = vec![];
        let num_sides = shape.get_side_count();
        let angle = std::f32::consts::TAU / num_sides as f32;
        for i in 0..num_sides {
            let cur_angle = angle * i as f32 + direction.angle();
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

    // Now add handle for the angles of the four different kinds of right triangles
    for rotation in [RightTriangleRotation::Zero, RightTriangleRotation::One, RightTriangleRotation::Two, RightTriangleRotation::Three] {
        let shape = TileShape::RightTriangle(rotation);
        let mut verticies = vec![[0.0, 0.0, 0.0], [-OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5, OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5, 0.0], [-OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5, -OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5, 0.0], [OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5, -OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        let uvs = vec![[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]];
        let indicies = vec![0, 1, 2, 0, 2, 3];
        for vertex in &mut verticies {
            *vertex = rotation.rotate(*vertex);
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

fn input_system(
    mut vis_state: ResMut<VisualState>,
    mut sim_state: ResMut<SimulationState>,

    mut input_state: ResMut<ui::InputState>,
    mouse_input: Res<Input<MouseButton>>,
    mouse_movements: EventReader<MouseMotion>,
    mouse_wheel_movements: EventReader<MouseWheel>,
    windows: Res<Windows>,
    ui_roots_query: Query<Entity, (With<ui::UiElement>, Without<Parent>)>,
    ui_element_query: Query<(&Transform, &mut ui::UiElement, Option<&Children>)>,
) {
    let processed_input = input_state.process_inputs(
        &mouse_input,
        mouse_movements,
        mouse_wheel_movements,
        &windows,
        ui_roots_query,
        ui_element_query,
    );

    if processed_input.over_some_ui {
        return;
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        vis_state.mouse_down = true;
        vis_state.mouse_moved = false;
    }

    vis_state.scale = (vis_state.scale + processed_input.scroll.y)
        .max(vis_state.min_scale)
        .min(vis_state.max_scale);

    if vis_state.mouse_down {
        if processed_input.movement.length_squared() > 0.001 {
            vis_state.mouse_moved = true;
            vis_state.cur_offset = sim_state.tiling.adjust_position(
                processed_input.movement * Vec2::new(-1.0, 1.0) / vis_state.scale
                    + vis_state.cur_offset,
            );
        }

        if mouse_input.just_released(MouseButton::Left) {
            vis_state.mouse_down = false;
            if !vis_state.mouse_moved {
                let primary_window = windows.primary();
                if let Some(position) = primary_window.cursor_position() {
                    let position =
                        position - Vec2::new(primary_window.width(), primary_window.height()) / 2.0;
                    let adjusted_position = position / vis_state.scale + vis_state.cur_offset;
                    let tile = sim_state.tiling.get_tile_containing(adjusted_position);
                    let target_state = (sim_state.get_at(tile.index) + 1) % sim_state.get_num_states_for_shape(tile.shape);
                    sim_state.set_at(tile.index, target_state);
                }
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
    app.add_plugin(
        ui::UIPlugin::new()
            .register_event::<menus::ChangeViewTo>()
            .register_event::<menus::ShowRulesFor>()
            .register_event::<menus::TogglePlay>()
            .register_event_generator::<menus::RuleUpdateEventGenerator>(),
    );
    app.add_plugin(menus::MenusPlugin);
    app.insert_resource(VisualsCache {
        meshes: Default::default(),
        states: Default::default(),
        outline_image: Default::default(),
        font: Handle::default(),
    })
    .insert_resource(SimulationState::new(Tiling {
        kind: TilingKind::Hexagonal,
        max_index: IVec2::new(50, 50),
        offset: Vec2::ZERO,
    }))
    .insert_resource(VisualState {
        mouse_down: false,
        mouse_moved: false,

        cur_offset: Vec2::ZERO,
        visual_grid_count: IVec2::new(52, 52),
        scale: 50.0,
        min_scale: 25.0,
        max_scale: 100.0,
        add_debug: false,
    })
    .add_startup_system(setup_world.after(menus::setup_menus))
    .add_system_to_stage(CoreStage::PreUpdate, input_system)
    .add_system(update_tile)
    .add_system(update_tile_visual.after(update_tile))
    .add_system(process_simulation)
    .run()
}
