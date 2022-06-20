use bevy::{
    hierarchy::DespawnRecursiveExt,
    math::{IVec2, Quat, Vec3, Vec3Swizzles, Vec2},
    pbr::StandardMaterial,
    prelude::{
        Assets, Color, Commands, Component, Entity, EventReader, Query, Res, ResMut, Transform, Handle, info,
    },
    utils::{HashMap, HashSet},
};

use crate::{
    hashmap_ext::HashMultiMapExt,
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
    position_to_entry: HashMap<CollapseEntryIndex, Entity>,
    max_height: u32,
    dual_tiling: Tiling,
    base_tiling: Tiling,
    collapsed_indicies: HashSet<(u32, IVec2)>,
    material: Handle<StandardMaterial>,

    height_updates: HashMap<IVec2, Vec<(IVec2, u32)>>,
    neighbor_restriction_updates: HashMap<CollapseEntryIndex, Vec<CollapseNeighborUpdate>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CollapseEntryIndex {
    index: IVec2,
    height: u32,
}

impl CollapseEntryIndex {
    pub const fn new(index: IVec2, height: u32) -> Self {
        Self { index, height }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CollapseNeighborUpdate {
    side: usize,
    walls: usize,
    offset: i32,
}

impl Default for CollapseState {
    fn default() -> Self {
        Self {
            position_to_entry: Default::default(),
            max_height: 0,
            dual_tiling: Tiling {
                kind: TilingKind::Square,
                max_index: IVec2::ZERO,
                offset: Vec2::ZERO,
            },
            base_tiling: Tiling {
                kind: TilingKind::Square,
                max_index: IVec2::ZERO,
                offset: Vec2::ZERO,
            },
            collapsed_indicies: Default::default(),
            material: Default::default(),
            height_updates: Default::default(),
            neighbor_restriction_updates: Default::default(),
        }
    }
}

#[derive(Component)]
pub struct CollapseEntry {
    pub index_in_tiling: IVec2,
    pub height: u32,
    pub options: usize,
    pub current_mesh: Option<GeometryHandle>,
    pub corner_data: Vec<(IVec2, u32)>,
    pub current_bottom_indicator: usize,
    pub current_top_indicator: usize,
    edge_restrictions: Vec<EdgeRestriction>,
    // Store the possible set of geometry handles from our corner handles alone. This get's modified
    // only when our corner data updates.
    pub possible_geometry_entries_from_corner_data: GeometryHandleSet,
    // Scratch spaced used in the wave function collapse algorithm. As our neighbors choose their
    // final meshes, this set decreases in size...
    pub geometry_entry_scratch: GeometryHandleSet,
}

pub struct EdgeRestriction {
    pub edge: usize,
    pub bottom_restriction: Option<usize>,
    pub level_restriction: Option<usize>,
    pub top_restriction: Option<usize>,
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
            options: 0,
            current_mesh: None,
            corner_data,
            current_bottom_indicator,
            current_top_indicator,
            edge_restrictions: Vec::new(),
            possible_geometry_entries_from_corner_data: possible_geometry_entries_from_corner_data
                .clone(),
            geometry_entry_scratch: possible_geometry_entries_from_corner_data,
        }
    }

    fn compute_edge_restrictions(&self, geom_data: &GeometryStorage) -> Vec<GeometryHandleSet> {
        let mut restrictions = Vec::new();
        for (side, edge_restriction) in self.edge_restrictions.iter().enumerate() {
            if let Some(walls) = edge_restriction.bottom_restriction {
                restrictions.push(geom_data.get_wall_union(self.corner_data.len(), side, walls));
            }
            if let Some(walls) = edge_restriction.level_restriction {
                restrictions.push(geom_data.get_wall_union(self.corner_data.len(), side, walls));
            }
            if let Some(walls) = edge_restriction.top_restriction {
                restrictions.push(geom_data.get_wall_union(self.corner_data.len(), side, walls));
            }
        }
        restrictions
    }

    fn recompute_from_restrictions(&mut self, mut select: bool, tiling: &Tiling, max_height: u32, geom_data: &GeometryStorage) -> Vec<(CollapseEntryIndex, CollapseNeighborUpdate)> {
        let edge_restrictions = self.compute_edge_restrictions(geom_data);
        let main_restriction = [&self.possible_geometry_entries_from_corner_data];
        let mut current_total_restrictions = GeometryHandleSet::intersection(main_restriction.into_iter().chain(&edge_restrictions));

        if current_total_restrictions.empty() {
            self.edge_restrictions.clear();
            current_total_restrictions = self.possible_geometry_entries_from_corner_data.clone();
        }

        // Check to see if our current handle still is in the set of our restrictions and if so
        // use that as our restriction instead of the restrictions from our edges and corners.
        if let Some(current) = self.current_mesh {
            if current_total_restrictions.contains(current) {
                current_total_restrictions = GeometryHandleSet::new(self.corner_data.len());
                current_total_restrictions.insert(current);
                select = false;
            } else {
                self.current_mesh = None;
            }
        }

        // If we need to select a mesh, then select one.
        if select {
            self.current_mesh = current_total_restrictions.into_iter().next();
            if let Some(current) = self.current_mesh {
                current_total_restrictions = GeometryHandleSet::new(self.corner_data.len());
                current_total_restrictions.insert(current);
            }
        }

        self.options = current_total_restrictions.unique_index_count();

        // Collect our updates from our current restrictions.
        let mut updates = Vec::with_capacity(3 * self.corner_data.len());
        let walls = geom_data.get_walls_in_set(&current_total_restrictions);
        for (side, (x_offset, y_offset, neighbor_side)) in tiling.get_adjacent(self.index_in_tiling).iter().enumerate() {
            let adjacent_index = self.index_in_tiling + IVec2::new(*x_offset, *y_offset);
            let (bottom_walls, level_walls, top_walls) = WallProfile::from_bits(walls[side]).iter().fold((0, 0, 0), |(bottom, level, top), wall| {
                (
                    bottom | wall.compatible_below(),
                    level | wall.compatible_across(),
                    top | wall.compatible_above(),
                )
            });
            if self.height + 1 < max_height {
                updates.push((CollapseEntryIndex::new(adjacent_index, self.height + 1), CollapseNeighborUpdate { side: *neighbor_side, walls: top_walls, offset: -1  }));
            }
            updates.push((CollapseEntryIndex::new(adjacent_index, self.height), CollapseNeighborUpdate { side: *neighbor_side, walls: level_walls, offset: 0  }));
            if self.height >= 1 {
                updates.push((CollapseEntryIndex::new(adjacent_index, self.height - 1), CollapseNeighborUpdate { side: *neighbor_side, walls: bottom_walls, offset: 1  }));
            }
        }

        updates
    }

    /// Update the state of this entry given a set of corner-height pair changes and return
    /// a list of neighbor updates that need to be propagated out to our neighbors. This is
    /// what will initiate the collapse and ensure we are in a "collapsable" state
    pub fn vertex_set_to(&mut self, corner_value_pairs: &[(IVec2, u32)], tiling: &Tiling, max_height: u32, geom_data: &GeometryStorage) -> Vec<(CollapseEntryIndex, CollapseNeighborUpdate)> {
        // First we are going to update our corner storage. If we already have set what
        // is passed into us then we will return and do nothing.
        let mut did_an_update_happen = false;
        for (corner, new_value) in corner_value_pairs {
            for (index, value) in self.corner_data.iter_mut() {
                if *index == *corner {
                    if *value != *new_value {
                        did_an_update_happen = true;
                        *value = *new_value;
                        break;
                    }
                }
            }
        }
        if !did_an_update_happen {
            return Vec::new();
        }

        // Next, we are going to recompute what our corners allow for in
        // terms of stacking entries ontop of one another.
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

        // Now we recompute the list of options we have not subject to any adjacent nodes.
        self.possible_geometry_entries_from_corner_data = geom_data.get_vertical_matching(
            self.corner_data.len(),
            self.current_bottom_indicator,
            self.current_top_indicator
        );

        // Then we are going to determine if we are compatible with the restrictions we have
        // on us from our current neighbors. And if not, we will clear those restrictions and
        // publish restrictions to our neighbors based on either the combined restriction
        // or only the restrictions from our corner heights.
        self.recompute_from_restrictions(false, tiling, max_height, geom_data)
    }

    pub fn process_neighbor_updates(
        &mut self,
        updates: Vec<CollapseNeighborUpdate>,
        tiling: &Tiling,
        max_height: u32, 
        geom_data: &GeometryStorage,
    ) -> Vec<(CollapseEntryIndex, CollapseNeighborUpdate)> {
        let mut has_some_updates = false;
        for update in updates {
            match self.edge_restrictions.binary_search_by(|edges| edges.edge.cmp(&update.side)) {
                Ok(matching_index) => {
                    match update.offset {
                        -1 => {
                            if self.edge_restrictions[matching_index].bottom_restriction != Some(update.walls) {
                                has_some_updates = true;
                                self.edge_restrictions[matching_index].bottom_restriction = Some(update.walls);
                            }
                        },
                        0 => {
                            if self.edge_restrictions[matching_index].level_restriction != Some(update.walls) {
                                has_some_updates = true;
                                self.edge_restrictions[matching_index].level_restriction = Some(update.walls);
                            }
                        },
                        1 => {
                            if self.edge_restrictions[matching_index].top_restriction != Some(update.walls) {
                                has_some_updates = true;
                                self.edge_restrictions[matching_index].top_restriction = Some(update.walls);
                            }
                        },
                        _ => {}
                    }
                },
                Err(insert_index) => self.edge_restrictions.insert(insert_index, EdgeRestriction {
                    edge: update.side,
                    bottom_restriction: if update.offset == -1 { Some(update.walls) } else { None },
                    level_restriction: if update.offset == 0 { Some(update.walls) } else { None },
                    top_restriction: if update.offset == 1 { Some(update.walls) } else { None },
                })
            }
        }

        // If we have not modified our restrictions, then do not send back out updates. This protects
        // against our neighbors selecting their final meshes, causing us to recognize that and selecting
        // our final mesh and sending them updates, etc.
        if has_some_updates {
            self.recompute_from_restrictions(false, tiling, max_height, geom_data)
        } else {
            Vec::new()
        }
    }
}

pub fn rebuild_visuals(
    mut collapse_state: ResMut<CollapseState>,
    mut events: EventReader<SimulationStateChanged>,
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
                collapse_state.max_height = sim_state.num_states as u32;
                collapse_state.base_tiling = sim_state.tiling.clone();
                collapse_state.dual_tiling = sim_state.tiling.get_dual();
                collapse_state.collapsed_indicies = HashSet::new();

                collapse_state.height_updates.clear();
                collapse_state.neighbor_restriction_updates.clear();

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
                            .insert(CollapseEntryIndex::new(tile.index, 0), entity);
                    }
                }

                for x in 0..sim_state.tiling.max_index.x {
                    for y in 0..sim_state.tiling.max_index.y {
                        let index = IVec2::new(x, y);
                        let state = sim_state.get_at(IVec2::new(x, y));
                        for vertex in sim_state.tiling.get_verticies(index, false) {
                            collapse_state.height_updates.add_element(vertex, (index, state));
                        }
                    }
                }
            }
            SimulationStateChanged::StatesChanged(changes) => {
                if collapse_state.dual_tiling.kind != TilingKind::Square {
                    continue;
                }

                for (corner, new_value) in changes {
                    for vertex in sim_state.tiling.get_verticies(*corner, false) {
                        collapse_state.height_updates.add_element(vertex, (*corner, *new_value));
                    }
                }
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
    for _ in 0..200 {
        // Try to take one height update out of our list of height updates.
        let index = collapse_state.height_updates.keys().next().cloned();
        if let Some(index) = index {
            if let Some(updates) = collapse_state.height_updates.remove(&index) {
                for entry_height in 0..collapse_state.max_height {
                    if let Some(entity) = collapse_state.position_to_entry.get(&CollapseEntryIndex::new(index, entry_height)) {
                        if let Ok((_, mut entry, _, _)) = entry_query.get_mut(*entity) {
                            let neighbor_updates = entry.vertex_set_to(&updates, &collapse_state.dual_tiling, collapse_state.max_height, &geom_data);
                            collapse_state.neighbor_restriction_updates.extend_elements(neighbor_updates);
                        }
                    }
                }
                continue;
            }
        }

        // Try to take neighbor updates and pass them to the collapse entry
        if let Some(index)  = collapse_state.neighbor_restriction_updates.keys().next().cloned() {
            if let Some(updates) = collapse_state.neighbor_restriction_updates.remove(&index) {
                if let Some(entity) = collapse_state.position_to_entry.get(&index) {
                    if let Ok((_, mut entry, _, _)) = entry_query.get_mut(*entity) {
                        let neighbor_updates = entry.process_neighbor_updates(updates, &collapse_state.dual_tiling, collapse_state.max_height, &geom_data);
                        collapse_state.neighbor_restriction_updates.extend_elements(neighbor_updates);
                    }
                }
            }
        }

        // Now check elements that we need to select.
        let mut smallest_num = usize::MAX;
        let mut index = (0, IVec2::new(-1, -1));
        let mut entity_to_collapse = None;
        entry_query.for_each(|(entity, entry, _, _)| {
            if entry.current_mesh.is_some()
            {
                return;
            }
            if entry.options < smallest_num {
                smallest_num = entry.options;
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
        if let Ok((_, mut entry, mut mesh_instance, mut transform)) =
            entry_query.get_mut(entity_to_collapse)
        {
            let new_restrictions = entry.recompute_from_restrictions(true, &collapse_state.dual_tiling, collapse_state.max_height, &geom_data);
            collapse_state.neighbor_restriction_updates.extend_elements(new_restrictions);
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
        }
    }
}
