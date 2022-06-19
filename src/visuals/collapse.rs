use bevy::{
    hierarchy::DespawnRecursiveExt,
    math::{IVec2, Quat, Vec3, Vec3Swizzles},
    pbr::StandardMaterial,
    prelude::{
        Assets, Color, Commands, Component, Entity, EventReader, Query, Res, ResMut, Transform, Handle,
    },
    utils::{HashMap, HashSet},
};

use crate::{
    simulation::SimulationState,
    tiling::{Tiling, TilingKind},
};

use super::{
    geom::{GeomOrientation, GeometryHandle, GeometryStorage, VerticalProfile, WallProfile, handles::GeometryHandleSet},
    render::{instanced_mesh::MeshInstance, InstancedPbrBundle},
};

#[derive(Component)]
pub enum SimulationStateChanged {
    NewTiling,
    StatesChanged(Vec<(IVec2, u32)>),
}

pub struct CollapseState {
    pub position_to_entry: HashMap<(u32, IVec2), Entity>,
    pub dual_tiling: Tiling,
    pub collapsed_indicies: HashSet<(u32, IVec2)>,
    pub material: Handle<StandardMaterial>,
}

#[derive(Component)]
pub struct CollapseEntry {
    pub index_in_tiling: IVec2,
    pub height: u32,
    pub current_mesh: Option<GeometryHandle>,
    pub corner_data: Vec<(IVec2, u32)>,
    pub current_bottom_indicator: usize,
    pub current_top_indicator: usize,
    // Store the possible set of geometry handles from our corner handles alone. This get's modified
    // only when our corner data updates.
    pub possible_geometry_entries_from_corner_data: GeometryHandleSet,
    // Scratch spaced used in the wave function collapse algorithm. As our neighbors choose their
    // final meshes, this set decreases in size...
    pub geometry_entry_scratch: GeometryHandleSet,
}

impl CollapseEntry {
    pub fn new(
        tiling: &Tiling,
        sim_state: &SimulationState,
        geom_data: &GeometryStorage,
        index: IVec2,
        height: u32,
    ) -> Self {
        let corner_data = tiling
            .get_verticies(index, true)
            .iter()
            .map(|index| {
                (
                    *index,
                    if sim_state.tiling.in_bounds(*index) {
                        sim_state.get_at(*index)
                    } else {
                        0
                    },
                )
            })
            .collect::<Vec<_>>();

        let current_bottom_indicator = VerticalProfile::compute_indicator(
            &(corner_data.iter().map(|(_, h)| {
                if *h < height {
                    VerticalProfile::Empty
                } else if *h == height {
                    VerticalProfile::Full
                } else {
                    VerticalProfile::Stackable
                }
            }))
            .collect(),
            super::geom::GeomOrientation::Standard { rotations: 0 },
        );

        let current_top_indicator = VerticalProfile::compute_indicator(
            &(corner_data.iter().map(|(_, h)| {
                if *h < height + 1 {
                    VerticalProfile::Empty
                } else if *h == height + 1 {
                    VerticalProfile::Full
                } else {
                    VerticalProfile::Stackable
                }
            }))
            .collect(),
            super::geom::GeomOrientation::Standard { rotations: 0 },
        );

        let possible_geometry_entries_from_corner_data = geom_data.get_vertical_matching(
            corner_data.len(),
            current_bottom_indicator,
            current_top_indicator
        );

        Self {
            index_in_tiling: index,
            height,
            current_mesh: None,
            corner_data,
            current_bottom_indicator,
            current_top_indicator,
            possible_geometry_entries_from_corner_data: possible_geometry_entries_from_corner_data
                .clone(),
            geometry_entry_scratch: possible_geometry_entries_from_corner_data,
        }
    }

    pub fn vertex_set_to(&mut self, corner: IVec2, new_value: u32, geom_data: &GeometryStorage) {
        for (index, value) in self.corner_data.iter_mut() {
            if *index == corner {
                *value = new_value;
                break;
            }
        }
        let bottom_sequence = self
            .corner_data
            .iter()
            .map(|(_, h)| {
                if *h < self.height {
                    VerticalProfile::Empty
                } else if *h == self.height {
                    VerticalProfile::Full
                } else {
                    VerticalProfile::Stackable
                }
            })
            .collect();
        self.current_bottom_indicator = VerticalProfile::compute_indicator(
            &bottom_sequence,
            super::geom::GeomOrientation::Standard { rotations: 0 },
        );

        let top_sequence = self
            .corner_data
            .iter()
            .map(|(_, h)| {
                if *h < self.height + 1 {
                    VerticalProfile::Empty
                } else if *h == self.height + 1 {
                    VerticalProfile::Full
                } else {
                    VerticalProfile::Stackable
                }
            })
            .collect();
        self.current_top_indicator = VerticalProfile::compute_indicator(
            &top_sequence,
            super::geom::GeomOrientation::Standard { rotations: 0 },
        );

        let mut labels = String::new();
        for vertical_block in &bottom_sequence {
            labels.push_str(vertical_block.label());
        }
        labels.push('_');
        for vertical_block in &top_sequence {
            labels.push_str(vertical_block.label());
        }

        self.possible_geometry_entries_from_corner_data = geom_data.get_vertical_matching(
            self.corner_data.len(),
            self.current_bottom_indicator,
            self.current_top_indicator
        );
        self.geometry_entry_scratch = self.possible_geometry_entries_from_corner_data.clone();
    }

    pub fn neighbor_set_to(
        &mut self,
        geom_data: &GeometryStorage,
        neighbor_index: usize,
        walls: usize,
    ) -> bool {
        let new_possible = &self.geometry_entry_scratch & &geom_data.get_wall_union(self.corner_data.len(), neighbor_index, walls);
        if !new_possible.empty() {
            self.geometry_entry_scratch = new_possible;
        }
        self.geometry_entry_scratch.empty()
    }

    // Select a visual from the possible geometry entries you have
    pub fn select(
        &mut self,
        tiling: &Tiling,
        geom_data: &GeometryStorage,
    ) -> Vec<(IVec2, usize, WallProfile)> {
        if let Some(handle) = self
            .possible_geometry_entries_from_corner_data
            .into_iter()
            .next()
        {
            self.current_mesh = Some(handle);
            tiling
                .get_adjacent(self.index_in_tiling)
                .iter()
                .enumerate()
                .filter_map(|(side, (x, y, neighbor_side))| {
                    let neighbor_index = self.index_in_tiling + IVec2::new(*x, *y);
                    if tiling.in_bounds(neighbor_index) {
                        Some((
                            neighbor_index,
                            *neighbor_side,
                            geom_data.profiles[handle.index].get_wall(side, handle.orientation),
                        ))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            self.current_mesh = None;
            Vec::new()
        }
    }
}

pub fn rebuild_visuals(
    mut collapse_state: ResMut<CollapseState>,
    mut events: EventReader<SimulationStateChanged>,
    mut entry_query: Query<&mut CollapseEntry>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_state: Res<SimulationState>,
    geom_data: Res<GeometryStorage>,
    mut commands: Commands,
) {
    for evt in events.iter() {
        match evt {
            // In the case of a new tiling, build out an entirely new set of collapse entries to handle the tiling.
            SimulationStateChanged::NewTiling => {
                for entity in collapse_state.position_to_entry.values() {
                    commands.entity(*entity).despawn_recursive();
                }

                collapse_state.position_to_entry.clear();
                collapse_state.dual_tiling = sim_state.tiling.get_dual();
                collapse_state.collapsed_indicies = HashSet::new();

                if collapse_state.dual_tiling.kind != TilingKind::Square {
                    continue;
                }

                if collapse_state.material == Default::default() {
                    collapse_state.material = materials.add(StandardMaterial {
                        base_color: Color::INDIGO,
                        perceptual_roughness: 1.0,
                        double_sided: true,
                        ..Default::default()
                    });
                }

                for x in 0..collapse_state.dual_tiling.max_index.x {
                    for y in 0..collapse_state.dual_tiling.max_index.y {
                        let tile = collapse_state
                            .dual_tiling
                            .get_tile_at_index(IVec2::new(x, y));
                        let entity = commands
                            .spawn_bundle(InstancedPbrBundle {
                                transform: Transform::from_translation(
                                    tile.position.extend(0.0).xzy(),
                                ),
                                material: collapse_state.material.clone(),
                                ..Default::default()
                            })
                            .insert(CollapseEntry::new(
                                &collapse_state.dual_tiling,
                                &sim_state,
                                &geom_data,
                                tile.index,
                                0u32,
                            ))
                            .id();
                        collapse_state
                            .position_to_entry
                            .insert((0, tile.index), entity);
                    }
                }
            }
            SimulationStateChanged::StatesChanged(changes) => {
                if collapse_state.dual_tiling.kind != TilingKind::Square {
                    continue;
                }

                for (corner, new_value) in changes {
                    for entry_index in sim_state.tiling.get_verticies(*corner, false) {
                        if let Some(entity) =
                            collapse_state.position_to_entry.get(&(0, entry_index))
                        {
                            if let Ok(mut entry) = entry_query.get_mut(*entity) {
                                entry.vertex_set_to(*corner, *new_value, &geom_data);
                            }
                        }
                    }
                }
                entry_query.for_each_mut(|mut entry| {
                    entry.geometry_entry_scratch =
                        entry.possible_geometry_entries_from_corner_data.clone();
                });
                collapse_state.collapsed_indicies = HashSet::new();
            }
        }
    }
}

pub fn collapse_visuals(
    mut collapse_state: ResMut<CollapseState>,
    mut entry_query: Query<(
        Entity,
        &mut CollapseEntry,
        &mut MeshInstance,
        &mut Transform,
    )>,
    geom_data: Res<GeometryStorage>,
) {
    for _ in 0..20 {
        // Early out for it we have already completely collapsed the waveform.
        if collapse_state.collapsed_indicies.len() >= collapse_state.position_to_entry.len() {
            return;
        }

        let mut smallest_num = usize::MAX;
        let mut index = (0, IVec2::new(-1, -1));
        let mut entity_to_collapse = None;
        entry_query.for_each(|(entity, entry, _, _)| {
            if collapse_state
                .collapsed_indicies
                .contains(&(entry.height, entry.index_in_tiling))
            {
                return;
            }
            if entry.geometry_entry_scratch.unique_index_count() < smallest_num {
                smallest_num = entry.geometry_entry_scratch.unique_index_count();
                index = (entry.height, entry.index_in_tiling);
                entity_to_collapse = Some(entity);
            }
        });

        // Sanity check.
        if smallest_num == usize::MAX {
            return;
        }

        let entity_to_collapse = entity_to_collapse
            .expect("Somehow we had more indicies to collapse but did not find one to");
        let neighbor_updates = if let Ok((_, mut entry, mut mesh_instance, mut transform)) =
            entry_query.get_mut(entity_to_collapse)
        {
            let output = entry.select(&collapse_state.dual_tiling, &geom_data);
            if let Some(current_mesh) = entry.current_mesh {
                if let Some(new_handle) = &geom_data.mesh_handles[current_mesh.index] {
                    if new_handle.clone() != mesh_instance.mesh.clone() {
                        mesh_instance.mesh = new_handle.clone();
                    }
                }

                match current_mesh.orientation {
                    GeomOrientation::Standard { rotations } => {
                        transform.rotation = Quat::from_rotation_y(
                            -std::f32::consts::TAU * rotations as f32
                                / collapse_state
                                    .dual_tiling
                                    .get_tile_at_index(entry.index_in_tiling)
                                    .shape
                                    .get_side_count() as f32,
                        );
                        transform.scale = Vec3::new(1.0, 1.0, 1.0);
                    }
                    GeomOrientation::Flipped { rotations } => {
                        transform.rotation = Quat::from_rotation_y(
                            -std::f32::consts::TAU * rotations as f32
                                / collapse_state
                                    .dual_tiling
                                    .get_tile_at_index(entry.index_in_tiling)
                                    .shape
                                    .get_side_count() as f32,
                        );
                        transform.scale = Vec3::new(1.0, 1.0, -1.0);
                    }
                }
            }

            output
        } else {
            Vec::new()
        };

        collapse_state.collapsed_indicies.insert(index);

        for (neighbor_index, neighbor_side, wall) in neighbor_updates {
            // Ignore states we have already covered
            if collapse_state
                .collapsed_indicies
                .contains(&(index.0, neighbor_index))
            {
                continue;
            }
            if let Some(entity) = collapse_state
                .position_to_entry
                .get(&(index.0, neighbor_index))
            {
                if let Ok((_, mut entry, _, _)) = entry_query.get_mut(*entity) {
                    entry.neighbor_set_to(&geom_data, neighbor_side, wall.to_bits());
                }
            }
        }
    }
}
