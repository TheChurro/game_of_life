use bevy::{
    hierarchy::{BuildChildren, Children, Parent},
    input::{
        mouse::{MouseMotion, MouseWheel},
        Input,
    },
    math::{IVec2, Mat4, Quat, Vec2, Vec3},
    pbr::{
        AmbientLight, DirectionalLight, DirectionalLightBundle,
        StandardMaterial,
    },
    prelude::{
        App, AssetServer, Assets, Camera, Changed, Color, Commands, Component, CoreStage,
        Entity, EventReader, EventWriter, GlobalTransform, Handle, Image, KeyCode, Mesh,
        MouseButton, OrthographicCameraBundle, ParallelSystemDescriptorCoercion,
        PerspectiveCameraBundle, Query, Res, ResMut, Transform, Visibility, With, Without,
    },
    render::{
        camera::Camera3d,
        mesh::{Indices, PrimitiveTopology},
    },
    sprite::{ColorMaterial, MaterialMesh2dBundle, Mesh2dHandle},
    text::{Font, Text, Text2dBundle, TextAlignment, TextSection, TextStyle},
    utils::HashMap,
    window::Windows,
    DefaultPlugins,
};

use menus::{DebugTileEvent, MenuState};
use simulation::SimulationState;
use tiling::{
    EquilateralDirection, RightTriangleRotation, TileShape, Tiling, TilingKind,
    OCTAGON_SQUARE_DIFFERENCE_OF_CENTER,
};
use visuals::{
    collapse::{
        collapse_visuals, rebuild_visuals, CollapseState,
        SimulationStateChanged,
    },
    render::{
        instanced_mesh::InstanceMeshRenderPlugin, instanced_mesh_material::InstancedMaterialPlugin,
        instanced_pbr::InstancedStandardMaterial,
    },
};

extern crate bevy;
extern crate bevy_obj;
extern crate bitflags;
extern crate bytemuck;
extern crate enum_flags;

mod hashmap_ext;
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
    camera_offset: Vec3,
    camera_angle: Vec2,
    last_click_pos: Option<Vec3>,
    visual_grid_count: IVec2,
    scale: f32,
    min_scale: f32,
    max_scale: f32,
    add_debug: bool,
    hide: bool,
}

#[derive(Component)]
pub struct VisualsCache {
    meshes: HashMap<TileShape, Mesh2dHandle>,
    states: HashMap<u32, Handle<ColorMaterial>>,
    outline_image: Handle<Image>,
    font: Handle<Font>,
    debug_vis_material: Handle<StandardMaterial>,
    debug_vis_outline_material: Handle<StandardMaterial>,
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
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
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
    for rotation in [
        RightTriangleRotation::Zero,
        RightTriangleRotation::One,
        RightTriangleRotation::Two,
        RightTriangleRotation::Three,
    ] {
        let shape = TileShape::RightTriangle(rotation);
        let mut verticies = vec![
            [0.0, 0.0, 0.0],
            [
                -OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5,
                OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5,
                0.0,
            ],
            [
                -OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5,
                -OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5,
                0.0,
            ],
            [
                OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5,
                -OCTAGON_SQUARE_DIFFERENCE_OF_CENTER * 0.5,
                0.0,
            ],
        ];
        let normals = vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ];
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

    commands.insert_resource(AmbientLight {
        color: Color::ORANGE_RED,
        brightness: 0.02,
    });

    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_8),
            ..Default::default()
        },
        ..Default::default()
    });

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

    visuals_cache.debug_vis_material = standard_materials.add(StandardMaterial {
        cull_mode: None,
        ..Color::WHITE.into()
    });
    visuals_cache.debug_vis_outline_material = standard_materials.add(StandardMaterial {
        cull_mode: None,
        double_sided: true,
        ..Color::BLACK.into()
    });

    let mut camera = PerspectiveCameraBundle::new_3d();
    camera.transform =
        Transform::from_xyz(-20.0, 20.0, -20.0).looking_at(Vec3::new(25.0, 0.0, 25.0), Vec3::Y);
    commands.spawn_bundle(camera);
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn update_tile(
    mut tile_query: Query<(&mut Transform, &mut TileState, &mut Visibility)>,
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
    tile_query.for_each_mut(|(mut transform, mut state, mut vis)| {
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

        if vis.is_visible == vis_state.hide {
            vis.is_visible = !vis_state.hide;
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
    collapse_state: ResMut<CollapseState>,

    keyboard: Res<Input<KeyCode>>,
    mut input_state: ResMut<ui::InputState>,
    mouse_input: Res<Input<MouseButton>>,
    mouse_movements: EventReader<MouseMotion>,
    mouse_wheel_movements: EventReader<MouseWheel>,
    windows: Res<Windows>,
    ui_roots_query: Query<Entity, (With<ui::UiElement>, Without<Parent>)>,
    ui_element_query: Query<(&Transform, &mut ui::UiElement, Option<&Children>)>,
    camera: Query<(&GlobalTransform, &Camera), With<Camera3d>>,
    mut inspect_events: EventWriter<DebugTileEvent>,
) {
    let shift_down = keyboard.pressed(KeyCode::LShift) || keyboard.pressed(KeyCode::RShift);
    let processed_input = input_state.process_inputs(
        shift_down,
        &mouse_input,
        mouse_movements,
        mouse_wheel_movements,
        &windows,
        ui_roots_query,
        ui_element_query,
    );

    if !input_state.has_selection() {
        if keyboard.just_pressed(KeyCode::H) && !vis_state.mouse_down {
            vis_state.hide = !vis_state.hide;
        }
    }

    if processed_input.over_some_ui {
        return;
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        vis_state.mouse_down = true;
        vis_state.mouse_moved = false;
        vis_state.last_click_pos = None;
    }

    vis_state.scale = (vis_state.scale + processed_input.scroll.y)
        .max(vis_state.min_scale)
        .min(vis_state.max_scale);

    if vis_state.mouse_down {
        let primary_window = windows.primary();
        if let Some(mouse_pos) = windows.primary().cursor_position() {
            if vis_state.hide {
                if let Ok((transform, camera)) = camera.get_single() {
                    let camera_transform: Mat4 = transform.compute_matrix();
                    let camera_matrix: Mat4 = camera.projection_matrix.inverse();
                    let view_matrix = camera_transform * camera_matrix;

                    let x = 2.0 * (mouse_pos.x / primary_window.width() as f32) - 1.0;
                    let y = 2.0 * (mouse_pos.y / primary_window.height() as f32) - 1.0;

                    let near = view_matrix.project_point3(Vec3::new(x, y, -1.0));
                    let far = view_matrix.project_point3(Vec3::new(x, y, 1.0));

                    let dir = (far - near).normalize();

                    let new_pos = if dir.y.signum() != near.y.signum() {
                        let time_to_plane = near.y / -dir.y;
                        Some(near + dir * time_to_plane)
                    } else {
                        None
                    };

                    if shift_down {
                        if processed_input.movement.length_squared() > 0.01 || vis_state.mouse_moved
                        {
                            vis_state.mouse_moved = true;
                            vis_state.camera_angle += processed_input.movement;
                            vis_state.camera_angle.y = vis_state.camera_angle.y.clamp(5.0, 85.0);
                            vis_state.camera_angle.x = vis_state.camera_angle.x % 360.0;
                        }
                    } else {
                        if let Some(last_pos) = vis_state.last_click_pos {
                            let offset = last_pos - new_pos.unwrap_or(last_pos);
                            if vis_state.mouse_moved || offset.length_squared() > 0.1 {
                                vis_state.camera_offset += offset * 0.5;
                                vis_state.last_click_pos = new_pos;
                                vis_state.mouse_moved = true;
                            }
                        } else {
                            vis_state.last_click_pos = new_pos;
                        }
                    }

                    if mouse_input.just_released(MouseButton::Left) {
                        vis_state.mouse_down = false;
                        if !vis_state.mouse_moved {
                            if let Some(pos) = new_pos {
                                if shift_down
                                {
                                    let tile = collapse_state
                                        .dual_tiling
                                        .get_tile_containing(Vec2::new(pos.x, pos.z));
                                    inspect_events.send(DebugTileEvent(tile.index));
                                } else {
                                    let tile = sim_state
                                        .tiling
                                        .get_tile_containing(Vec2::new(pos.x, pos.z));
                                    let target_state = (sim_state.get_at(tile.index) + 1)
                                        % sim_state.get_num_states_for_shape(tile.shape);
                                    sim_state.set_at(tile.index, target_state);
                                }
                            }
                        }
                    }
                }
            } else {
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
                        let mouse_pos = mouse_pos
                            - Vec2::new(primary_window.width(), primary_window.height()) / 2.0;
                        let adjusted_position = mouse_pos / vis_state.scale + vis_state.cur_offset;
                        let tile = sim_state.tiling.get_tile_containing(adjusted_position);
                        let target_state = (sim_state.get_at(tile.index) + 1)
                            % sim_state.get_num_states_for_shape(tile.shape);
                        sim_state.set_at(tile.index, target_state);
                    }
                }
            }
        }
    }
}

fn process_simulation(
    mut sim_state: ResMut<SimulationState>,
    mut events: EventWriter<SimulationStateChanged>,
) {
    let changes = sim_state.process();
    if changes.len() > 0 {
        events.send(SimulationStateChanged::StatesChanged(changes));
    }
}

fn move_camera(vis_state: Res<VisualState>, mut camera: Query<&mut Transform, With<Camera3d>>) {
    camera.for_each_mut(|mut transform| {
        *transform = Transform::from_translation(
            Vec3::new(
                vis_state.scale
                    * vis_state.camera_angle.x.to_radians().cos()
                    * vis_state.camera_angle.y.to_radians().cos(),
                vis_state.scale * vis_state.camera_angle.y.to_radians().sin(),
                vis_state.scale
                    * vis_state.camera_angle.x.to_radians().sin()
                    * vis_state.camera_angle.y.to_radians().cos(),
            ) + vis_state.camera_offset,
        )
        .looking_at(vis_state.camera_offset, Vec3::Y);
    });
}

fn main() {
    let mut app = App::new();
    let tiling = Tiling {
        kind: TilingKind::Square,
        max_index: IVec2::new(50, 50),
        offset: Vec2::ZERO,
    };
    app.add_plugins(DefaultPlugins);
    app.add_plugin(bevy_obj::ObjPlugin);
    app.add_plugin(
        ui::UIPlugin::new()
            .register_event::<menus::ChangeViewTo>()
            .register_event::<menus::ShowRulesFor>()
            .register_event::<menus::TogglePlay>()
            .register_event::<menus::DebugTileEvent>()
            .register_number_event_generator::<menus::RuleUpdateEventGenerator>()
            .register_text_event_generator::<menus::CommandEventGenerator>(),
    );
    app.add_plugin(menus::MenusPlugin);
    app.add_plugin(InstanceMeshRenderPlugin);
    app.add_plugin(InstancedMaterialPlugin::<InstancedStandardMaterial>::default());
    app.insert_resource(VisualsCache {
        meshes: Default::default(),
        states: Default::default(),
        outline_image: Default::default(),
        font: Handle::default(),
        debug_vis_material: Handle::default(),
        debug_vis_outline_material: Handle::default(),
    })
    .insert_resource(SimulationState::new(tiling))
    .insert_resource(VisualState {
        mouse_down: false,
        mouse_moved: false,

        cur_offset: Vec2::ZERO,
        camera_offset: Vec3::ZERO,
        camera_angle: Vec2::new(0.0, 20.0),
        last_click_pos: None,
        visual_grid_count: IVec2::new(26, 26),
        scale: 50.0,
        min_scale: 5.0,
        max_scale: 100.0,
        add_debug: false,
        hide: true,
    })
    .insert_resource(CollapseState::default())
    .add_event::<SimulationStateChanged>()
    .insert_resource(visuals::geom::GeometryStorage::new())
    .add_startup_system(setup_world.after(menus::setup_menus))
    .add_system_to_stage(CoreStage::PreUpdate, input_system)
    .add_startup_system(visuals::geom::load_geometry)
    .add_system(visuals::geom::log_geometry)
    .add_system(update_tile)
    .add_system(update_tile_visual.after(update_tile))
    .add_system(process_simulation)
    .add_system(collapse_visuals)
    .add_system(rebuild_visuals)
    .add_system(move_camera)
    .add_system(visuals::geom::geometry_input)
    .run()
}
