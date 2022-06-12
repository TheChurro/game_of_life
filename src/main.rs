use bevy::{
    input::{
        mouse::{MouseMotion, MouseWheel},
        Input,
    },
    math::{vec2, IVec2, Quat, Vec2, Vec3, Vec3Swizzles},
    prelude::{
        App, AssetServer, Assets, Changed, Color, Commands, Component, EventReader, Handle,
        KeyCode, Mesh, MouseButton, OrthographicCameraBundle, ParallelSystemDescriptorCoercion,
        Query, Res, ResMut, Transform, Visibility, info,
    },
    render::mesh::{Indices, PrimitiveTopology},
    sprite::{ColorMaterial, MaterialMesh2dBundle, Mesh2dHandle},
    text::{Font, Text, Text2dBundle, TextAlignment, TextSection, TextStyle},
    utils::HashMap,
    window::Windows,
    DefaultPlugins,
};
use simulation::SimulationState;
use tiling::{
    get_hexagon_band_perpendicular_to, TileShape, Tiling, TilingKind, HEXAGON_AXIS_LEFT,
    HEXAGON_AXIS_RIGHT, OCTAGON_SQUARE_DIFFERENCE_OF_CENTER,
};

extern crate bevy;

mod simulation;
mod tiling;

#[derive(Component)]
struct VisualState {
    cur_offset: Vec2,
    visual_grid_count: IVec2,
    scale: f32,
    min_scale: f32,
    max_scale: f32,
}

#[derive(Component)]
struct VisualsCache {
    meshes: HashMap<TileShape, Mesh2dHandle>,
    states: HashMap<u32, Handle<ColorMaterial>>,
    font: Handle<Font>,
}

#[derive(Component)]
struct TileState {
    offset_from_center: IVec2,
    computed_index: IVec2,
    current_state: u32,
    previous_shape: TileShape,
}

#[derive(Component)]
struct DebugComputation {
    pub position: Vec2,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DebugVisValue {
    Band,
    Segment,
    SquareInOctagonX,
    SquareInOctagonY,
    OctagonInOctagonX,
    OctagonInOctagonY,
}

struct DebugSettings {
    show: bool,
    axis: Vec2,
    debug_vis: DebugVisValue,
    density: i32,
    do_update: bool,
    enabled: bool,
}

struct InputState {
    is_mouse_down: bool,
    mouse_moved: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            is_mouse_down: false,
            mouse_moved: false,
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
    debug_settings: Res<DebugSettings>,
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

    visuals_cache.states.insert(
        0,
        materials.add(ColorMaterial {
            color: Color::rgb(0.5, 0.5, 0.5),
            texture: Some(outline_img.clone()),
        }),
    );
    visuals_cache.states.insert(
        1,
        materials.add(ColorMaterial {
            color: Color::rgb(0.0, 0.0, 0.0),
            texture: Some(outline_img.clone()),
        }),
    );

    visuals_cache.font =
        asset_server.load("fonts/brass-mono-font-freeware-peter-fonseca/BrassMonoRegular-o2Yz.otf");

    let default_color = visuals_cache
        .states
        .get(&0)
        .expect("Failed to get material just created!")
        .clone();

    let half_size = Vec2::ZERO; //sim_state.tiling.size() / 2.0;
    for x in -vis_state.visual_grid_count.x / 2..(vis_state.visual_grid_count.x + 1) / 2 {
        for y in -vis_state.visual_grid_count.y / 2..(vis_state.visual_grid_count.y + 1) / 2 {
            let tile = sim_state.tiling.get_tile_at_index(IVec2::new(x, y));

            commands
                .spawn_bundle(MaterialMesh2dBundle {
                    mesh: visuals_cache
                        .meshes
                        .get(&tile.shape)
                        .expect("Failed to get mesh we just inserted!")
                        .clone(),
                    material: default_color.clone(),
                    transform: Transform::from_translation((tile.position - half_size).extend(0.0)),
                    ..Default::default()
                })
                .insert(TileState {
                    offset_from_center: IVec2::new(x, y),
                    computed_index: sim_state.tiling.adjust_index(IVec2::new(x, y)),
                    current_state: if x == 25 { 1 } else { 0 },
                    previous_shape: tile.shape,
                });
        }
    }

    if debug_settings.enabled {
        let width = if sim_state.tiling.kind == TilingKind::Hexagonal {
            TileShape::Hexagon.get_width()
        } else {
            OCTAGON_SQUARE_DIFFERENCE_OF_CENTER
        };
        let height = if sim_state.tiling.kind == TilingKind::Hexagonal {
            TileShape::Hexagon.get_height()
        } else {
            OCTAGON_SQUARE_DIFFERENCE_OF_CENTER
        };
        for x in -vis_state.visual_grid_count.x * (debug_settings.density / 2)
            ..vis_state.visual_grid_count.x * (debug_settings.density / 2)
        {
            for y in -vis_state.visual_grid_count.y * (debug_settings.density / 2)
                ..vis_state.visual_grid_count.y * (debug_settings.density / 2)
            {
                commands
                    .spawn_bundle(Text2dBundle {
                        text: Text {
                            sections: Vec::new(),
                            alignment: TextAlignment {
                                vertical: bevy::text::VerticalAlign::Center,
                                horizontal: bevy::text::HorizontalAlign::Center,
                            },
                        },
                        visibility: Visibility { is_visible: false },
                        ..Default::default()
                    })
                    .insert(DebugComputation {
                        position: Vec2::new(
                            x as f32 / debug_settings.density as f32 * width,
                            y as f32 / debug_settings.density as f32 * height,
                        ),
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
        (&mut Mesh2dHandle, &mut Handle<ColorMaterial>, &TileState),
        Changed<TileState>,
    >,
    visuals_cache: Res<VisualsCache>,
    sim_state: Res<SimulationState>,
) {
    tile_query.for_each_mut(|(mut mesh, mut material, state)| {
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
    });
}

const SCROLL_SENSITIVITY: f32 = 0.5;

fn input_system(
    mouse_input: Res<Input<MouseButton>>,
    key_input: Res<Input<KeyCode>>,
    mut mouse_state: ResMut<InputState>,
    mut mouse_movements: EventReader<MouseMotion>,
    mut mouse_wheel_movements: EventReader<MouseWheel>,
    mut vis_state: ResMut<VisualState>,
    mut sim_state: ResMut<SimulationState>,
    mut debug_state: ResMut<DebugSettings>,
    windows: Res<Windows>,
) {
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

    for motion in mouse_wheel_movements.iter() {
        vis_state.scale = (vis_state.scale + motion.y * SCROLL_SENSITIVITY)
            .max(vis_state.min_scale)
            .min(vis_state.max_scale);
    }

    if mouse_input.just_released(MouseButton::Left) {
        mouse_state.is_mouse_down = false;
        if !mouse_state.mouse_moved {
            let primary_window = windows.primary();
            if let Some(position) = primary_window.cursor_position() {
                let position =
                    position - Vec2::new(primary_window.width(), primary_window.height()) / 2.0;
                let adjusted_position = position / vis_state.scale + vis_state.cur_offset;
                let index = sim_state.tiling.get_index_for_position(adjusted_position);
                let target_state = if sim_state.get_at(index) == 0 { 1 } else { 0 };
                sim_state.set_at(index, target_state);
            }
        }
    }

    if key_input.just_released(KeyCode::P) {
        let central_tile = sim_state.tiling.get_tile_containing(vis_state.cur_offset);
        let mut offset = central_tile.position - vis_state.cur_offset;
        // This is super hacky way to make sure we wrap smoothly but whatever...
        let tiling_size = sim_state.tiling.size();
        let offfset_pre_adjustment = offset;
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
        info!("Vis Offset {}\n    Central Tile Index: {}\n    Central Tile Pos: {}\n    Offset From Tile: {}\n    Adjusted Offset From Tile: {}", vis_state.cur_offset, central_tile.index, central_tile.position, offfset_pre_adjustment, offset);
    }

    if key_input.just_released(KeyCode::H) {
        *sim_state = SimulationState::new(Tiling {
            kind: TilingKind::Hexagonal,
            max_index: IVec2::new(50, 50),
        });
    } else if key_input.just_released(KeyCode::S) {
        *sim_state = SimulationState::new(Tiling {
            kind: TilingKind::Square,
            max_index: IVec2::new(50, 50),
        });
    } else if key_input.just_released(KeyCode::O) {
        *sim_state = SimulationState::new(Tiling {
            kind: TilingKind::OctagonAndSquare,
            max_index: IVec2::new(50, 50),
        });
    } else if key_input.just_released(KeyCode::Left) {
        debug_state.axis = Vec2::new(HEXAGON_AXIS_LEFT.cos(), HEXAGON_AXIS_LEFT.sin());
        debug_state.do_update = debug_state.show;
    } else if key_input.just_released(KeyCode::Right) {
        debug_state.do_update = debug_state.show;
        debug_state.axis = Vec2::new(HEXAGON_AXIS_RIGHT.cos(), HEXAGON_AXIS_RIGHT.sin());
    } else if key_input.just_released(KeyCode::Up) {
        debug_state.do_update = debug_state.show;
        debug_state.axis = Vec2::new(1.0, 0.0);
    } else if key_input.just_released(KeyCode::D) {
        debug_state.do_update = true;
        debug_state.show = !debug_state.show;
    } else if key_input.just_released(KeyCode::V) {
        debug_state.do_update = debug_state.show;
        debug_state.debug_vis = match sim_state.tiling.kind {
            TilingKind::Square => DebugVisValue::Band,
            TilingKind::Hexagonal => {
                if debug_state.debug_vis == DebugVisValue::Band {
                    DebugVisValue::Segment
                } else {
                    DebugVisValue::Band
                }
            }
            TilingKind::OctagonAndSquare => match debug_state.debug_vis {
                DebugVisValue::Band => DebugVisValue::SquareInOctagonX,
                DebugVisValue::Segment => DebugVisValue::SquareInOctagonX,
                DebugVisValue::SquareInOctagonX => DebugVisValue::OctagonInOctagonX,
                DebugVisValue::SquareInOctagonY => DebugVisValue::OctagonInOctagonY,
                DebugVisValue::OctagonInOctagonX => DebugVisValue::SquareInOctagonX,
                DebugVisValue::OctagonInOctagonY => DebugVisValue::SquareInOctagonY,
            },
        }
    } else if key_input.just_released(KeyCode::Down) {
        let old_do_update = debug_state.do_update;
        debug_state.do_update = debug_state.show;
        debug_state.debug_vis = match debug_state.debug_vis {
            DebugVisValue::SquareInOctagonX => DebugVisValue::SquareInOctagonY,
            DebugVisValue::SquareInOctagonY => DebugVisValue::SquareInOctagonX,
            DebugVisValue::OctagonInOctagonX => DebugVisValue::OctagonInOctagonY,
            DebugVisValue::OctagonInOctagonY => DebugVisValue::OctagonInOctagonX,
            x => {
                debug_state.do_update = old_do_update;
                x
            }
        }
    }
}

fn process_simulation(mut sim_state: ResMut<SimulationState>) {
    sim_state.process(false);
}

fn update_debug_component(
    mut query: Query<(
        &mut Transform,
        &mut Text,
        &mut Visibility,
        &DebugComputation,
    )>,
    mut params: ResMut<DebugSettings>,
    vis_state: Res<VisualState>,
    vis_cache: Res<VisualsCache>,
) {
    if !params.do_update {
        return;
    }
    params.do_update = false;
    query.for_each_mut(|(mut transform, mut text, mut visibility, data)| {
        if params.show {
            visibility.is_visible = true;
            transform.translation = (data.position * vis_state.scale).extend(1.0);

            text.sections.clear();
            if params.debug_vis == DebugVisValue::Band || params.debug_vis == DebugVisValue::Segment
            {
                let (band, segment, in_band) =
                    get_hexagon_band_perpendicular_to(data.position, params.axis);

                text.sections.push(TextSection {
                    value: match params.debug_vis {
                        DebugVisValue::Band => format!("{}", band),
                        DebugVisValue::Segment => format!("{}", segment),
                        _ => "E".into(),
                    },
                    style: TextStyle {
                        font: vis_cache.font.clone(),
                        font_size: 14.0,
                        color: if in_band { Color::GREEN } else { Color::RED },
                    },
                });
            } else if params.debug_vis == DebugVisValue::SquareInOctagonX
                || params.debug_vis == DebugVisValue::SquareInOctagonY
            {
                let small_indicies = ((data.position + Vec2::new(0.5, 0.5))
                    / OCTAGON_SQUARE_DIFFERENCE_OF_CENTER)
                    .floor()
                    .as_ivec2();
                let small_pos = small_indicies.as_vec2() * OCTAGON_SQUARE_DIFFERENCE_OF_CENTER;
                let pos = data.position + Vec2::new(0.5, 0.5) - small_pos;
                text.sections.push(TextSection {
                    value: format!(
                        "{}",
                        if params.debug_vis == DebugVisValue::SquareInOctagonX {
                            small_indicies.x
                        } else {
                            small_indicies.y
                        }
                    ),
                    style: TextStyle {
                        font: vis_cache.font.clone(),
                        font_size: 14.0,
                        color: if pos.x < 1.0
                            && pos.y < 1.0
                            && (small_indicies.x + small_indicies.y) % 2 == 0
                        {
                            Color::GREEN
                        } else {
                            Color::RED
                        },
                    },
                });
            } else if params.debug_vis == DebugVisValue::OctagonInOctagonX
                || params.debug_vis == DebugVisValue::OctagonInOctagonY
            {
                let rotated_position = (Quat::from_rotation_z(-std::f32::consts::FRAC_PI_4)
                    * data.position.extend(0.0))
                .xy();
                let rotated_index = (rotated_position / TileShape::Octagon.get_height())
                    .floor()
                    .as_ivec2();
                text.sections.push(TextSection {
                    value: format!(
                        "{}",
                        if params.debug_vis == DebugVisValue::OctagonInOctagonX {
                            rotated_index.x
                        } else {
                            rotated_index.y
                        }
                    ),
                    style: TextStyle {
                        font: vis_cache.font.clone(),
                        font_size: 14.0,
                        color: Color::GREEN,
                    },
                });
            }
        } else {
            visibility.is_visible = false;
        }
    });
}

fn main() {
    let test_tiling = Tiling {
        kind: TilingKind::Hexagonal,
        max_index: IVec2::new(2, 2),
    };
    assert_eq!(test_tiling.adjust_index(IVec2::new(1, 2)), IVec2::new(0, 0), "Adjustment was incorrect");
    assert_eq!(test_tiling.adjust_index(IVec2::new(2, 3)), IVec2::new(1, 1), "Adjustment was incorrect");
    assert_eq!(test_tiling.adjust_index(IVec2::new(2, 4)), IVec2::new(0, 0), "Adjustment was incorrect");
    let test_tiling = Tiling {
        kind: TilingKind::Hexagonal,
        max_index: IVec2::new(4, 4),
    };
    assert_eq!(test_tiling.adjust_index(IVec2::new(1, 2)), IVec2::new(1, 2), "Adjustment was incorrect");
    assert_eq!(test_tiling.adjust_index(IVec2::new(2, 4)), IVec2::new(0, 0), "Adjustment was incorrect");
    assert_eq!(test_tiling.adjust_index(IVec2::new(3, 4)), IVec2::new(1, 0), "Adjustment was incorrect");
    let test_tiling = Tiling {
        kind: TilingKind::Hexagonal,
        max_index: IVec2::new(50, 50),
    };    assert_eq!(test_tiling.get_index_for_position(Vec2::new(0.0, test_tiling.size().y - 0.5)), IVec2::new(0, 0), "Adjustment was incorrect");

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(VisualsCache {
            meshes: Default::default(),
            states: Default::default(),
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
        })
        .insert_resource(DebugSettings {
            show: false,
            axis: Vec2::new(1.0, 0.0),
            debug_vis: DebugVisValue::Band,
            density: 6,
            do_update: false,
            enabled: true,
        })
        .insert_resource(InputState::default())
        .add_startup_system(setup_world)
        .add_system(input_system)
        .add_system(update_tile.after(input_system))
        .add_system(update_tile_visual.after(update_tile))
        .add_system(process_simulation)
        .add_system(update_debug_component)
        .run()
}
