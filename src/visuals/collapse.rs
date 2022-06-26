use std::fmt::Display;

use bevy::{
    hierarchy::DespawnRecursiveExt,
    math::{IVec2, Vec2, Vec3Swizzles},
    prelude::{
        info, Assets, Color, Commands, Component, Entity, EventReader, Handle, Mut, Query, Res,
        ResMut, Transform,
    },
    utils::{HashMap, HashSet},
};

use crate::{
    hashmap_ext::HashMultiMapExt,
    simulation::SimulationState,
    tiling::{Tiling, TilingKind}, visuals::geom::build_profiles::WallProfileIndex, menus::DebugState,
};

use super::{
    geom::{
        handles::GeometryHandleSet, GeometryHandle, GeometryStorage, VerticalProfile,
    },
    render::{
        instanced_mesh::MeshInstance, instanced_pbr::InstancedStandardMaterial, InstancedPbrBundle,
    },
};

#[derive(Component)]
pub enum SimulationStateChanged {
    NewTiling,
    StatesChanged(Vec<(IVec2, u32)>),
}

pub struct CollapseState {
    pub position_to_entry: HashMap<CollapseEntryIndex, Entity>,
    max_height: u32,
    pub dual_tiling: Tiling,
    base_tiling: Tiling,
    collapsed_indicies: HashSet<(u32, IVec2)>,
    material: Handle<InstancedStandardMaterial>,

    height_updates: HashMap<IVec2, Vec<(IVec2, u32)>>,
    neighbor_restriction_updates: HashMap<CollapseEntryIndex, Vec<CollapseNeighborUpdate>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CollapseEntryIndex {
    pub index: IVec2,
    pub height: u32,
}

impl CollapseEntryIndex {
    pub const fn new(index: IVec2, height: u32) -> Self {
        Self { index, height }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CollapseNeighborUpdate {
    side: usize,
    walls: u128,
    #[cfg(debug_assertions)]
    from_neighbor: IVec2,
}

#[derive(Debug, Clone)]
pub enum CollapseHistory {
    SetCorner(IVec2, u32),
    SetEdge(
        #[cfg(debug_assertions)] IVec2,
        usize,
        u128,
        Option<u128>,
    ),
    Selected(GeometryHandle, usize),
    DownTo(GeometryHandle),
    SendingUpdates(IVec2, u32, u128),
    Deselected(bool),
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
    pub edge_restrictions: Vec<EdgeRestriction>,
    // Store the possible set of geometry handles from our corner handles alone. This get's modified
    // only when our corner data updates.
    pub possible_geometry_entries_from_corner_data: GeometryHandleSet,
    pub history: Vec<CollapseHistory>,
    pub history_enabled: bool,
}

pub struct EdgeRestriction {
    pub edge: usize,
    pub restruction: Option<u128>,
}

impl CollapseEntry {
    pub fn new(
        tiling: &Tiling,
        sim_state: &SimulationState,
        geom_data: &GeometryStorage,
        index: IVec2,
        height: u32,
        history_enabled: bool,
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
            current_top_indicator,
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
            history: Vec::new(),
            history_enabled,
        }
    }

    fn write_to_history(&mut self, history: CollapseHistory) {
        if self.history_enabled && self.history.len() < 100 {
            self.history.push(history);
        }
    }

    pub fn compute_edge_restrictions(&self, geom_data: &GeometryStorage) -> Vec<GeometryHandleSet> {
        let mut restrictions = Vec::new();
        for edge_restriction in &self.edge_restrictions {
            let mut restriction_bits = u128::MAX;
            if let Some(walls) = edge_restriction.restruction {
                restriction_bits &= walls;
            }
            restrictions.push(geom_data.get_wall_union(
                self.corner_data.len(),
                edge_restriction.edge,
                restriction_bits,
            ));
        }
        restrictions
    }

    pub fn compute_current_total_restriction(&self, geom_data: &GeometryStorage) -> GeometryHandleSet {
        let edge_restrictions = self.compute_edge_restrictions(geom_data);
        let main_restriction = [&self.possible_geometry_entries_from_corner_data];
        GeometryHandleSet::intersection(main_restriction.into_iter().chain(&edge_restrictions))
    }

    fn recompute_from_restrictions(
        &mut self,
        log_total_restrictions: bool,
        mut select: bool,
        tiling: &Tiling,
        #[allow(unused)]
        max_height: u32,
        geom_data: &GeometryStorage,
    ) -> Vec<(CollapseEntryIndex, CollapseNeighborUpdate)> {
        let edge_restrictions = self.compute_edge_restrictions(geom_data);
        let main_restriction = [&self.possible_geometry_entries_from_corner_data];
        let mut current_total_restrictions =
            GeometryHandleSet::intersection(main_restriction.into_iter().chain(&edge_restrictions));

        if log_total_restrictions {
            info!("  Total: {}", current_total_restrictions.data_string());
            for edge in &self.edge_restrictions {
                let mut total_restriction = u128::MAX;
                info!("  Edge {} restrictions:", edge.edge);
                if let Some(level) = edge.restruction {
                    let label = WallProfileIndex::from_bits(level)
                        .iter()
                        .map(|wall| wall.index().to_string())
                        .collect::<Vec<_>>()
                        .join(" ");
                    info!("    Restrictions: {}", label);
                    total_restriction &= level;
                }
                let walls =
                    geom_data.get_wall_union(self.corner_data.len(), edge.edge, total_restriction);
                for handle in &walls {
                    let mut data = format!("{}", handle);
                    let profile = &geom_data.profiles[handle.index];
                    for side in 0..profile.sides {
                        data.push_str(&geom_data.get_wall(profile, side, &handle.orientation).index().to_string());
                        data.push(' ');
                    }
                    info!("    Walls: {}", data);
                }
            }
        }

        if current_total_restrictions.empty() {
            self.edge_restrictions.clear();
            current_total_restrictions = self.possible_geometry_entries_from_corner_data.clone();
            self.write_to_history(CollapseHistory::Deselected(true));
            self.current_mesh = None;
        }

        if current_total_restrictions.length() == 1 {
            if let Some(handle) = current_total_restrictions.into_iter().next() {
                self.write_to_history(CollapseHistory::DownTo(handle));
            }
        }

        // Check to see if our current handle still is in the set of our restrictions and if so
        // use that as our restriction instead of the restrictions from our edges and corners.
        if let Some(current) = self.current_mesh {
            if current_total_restrictions.contains(current) {
                current_total_restrictions = GeometryHandleSet::new(self.corner_data.len());
                current_total_restrictions.insert(current);
                select = false;
            } else {
                self.write_to_history(CollapseHistory::Deselected(false));
                self.current_mesh = None;
            }
        }

        // If we need to select a mesh, then select one.
        if select {
            self.current_mesh = current_total_restrictions.into_iter().next();
            if let Some(current) = self.current_mesh {
                self.write_to_history(CollapseHistory::Selected(
                    current,
                    current_total_restrictions.length(),
                ));
                current_total_restrictions = GeometryHandleSet::new(self.corner_data.len());
                current_total_restrictions.insert(current);
            }
        }

        self.options = current_total_restrictions.length();

        // Collect our updates from our current restrictions.
        let mut updates = Vec::with_capacity(3 * self.corner_data.len());
        let walls = geom_data.get_walls_in_set(&current_total_restrictions);
        for (side, (x_offset, y_offset, neighbor_side)) in
            tiling.get_adjacent(self.index_in_tiling).iter().enumerate()
        {
            let adjacent_index = self.index_in_tiling + IVec2::new(*x_offset, *y_offset);
            if tiling.in_bounds(adjacent_index) {
                let opposite_walls = WallProfileIndex::from_bits(walls[side])
                    .iter()
                    .fold(0, |current, wall| {
                        current | geom_data.wall_profiles[wall.index()].reverse_profile.to_bits()
                    });
                updates.push((
                    CollapseEntryIndex::new(adjacent_index, self.height),
                    CollapseNeighborUpdate {
                        side: *neighbor_side,
                        walls: opposite_walls,
                        #[cfg(debug_assertions)]
                        from_neighbor: self.index_in_tiling,
                    },
                ));
                self.write_to_history(CollapseHistory::SendingUpdates(
                    adjacent_index,
                    self.height,
                    opposite_walls,
                ));
            }
        }

        updates
    }

    /// Update the state of this entry given a set of corner-height pair changes and return
    /// a list of neighbor updates that need to be propagated out to our neighbors. This is
    /// what will initiate the collapse and ensure we are in a "collapsable" state
    pub fn vertex_set_to(
        &mut self,
        log_total_restrictions: bool,
        corner_value_pairs: &[(IVec2, u32)],
        tiling: &Tiling,
        max_height: u32,
        geom_data: &GeometryStorage,
    ) -> Vec<(CollapseEntryIndex, CollapseNeighborUpdate)> {
        // First we are going to update our corner storage. If we already have set what
        // is passed into us then we will return and do nothing.
        let mut did_an_update_happen = false;
        for (corner, new_value) in corner_value_pairs {
            self.write_to_history(CollapseHistory::SetCorner(*corner, *new_value));
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
            self.current_top_indicator,
        );

        // Then we are going to determine if we are compatible with the restrictions we have
        // on us from our current neighbors. And if not, we will clear those restrictions and
        // publish restrictions to our neighbors based on either the combined restriction
        // or only the restrictions from our corner heights.
        self.recompute_from_restrictions(
            log_total_restrictions,
            false,
            tiling,
            max_height,
            geom_data,
        )
    }

    pub fn process_neighbor_updates(
        &mut self,
        log_total_restrictions: bool,
        updates: Vec<CollapseNeighborUpdate>,
        tiling: &Tiling,
        max_height: u32,
        geom_data: &GeometryStorage,
    ) -> Vec<(CollapseEntryIndex, CollapseNeighborUpdate)> {
        let mut has_some_updates = false;
        for update in updates {
            match self
                .edge_restrictions
                .binary_search_by(|edges| edges.edge.cmp(&update.side))
            {
                Ok(matching_index) => {
                    if self.edge_restrictions[matching_index].restruction
                            != Some(update.walls)
                    {
                        has_some_updates = true;
                        self.write_to_history(CollapseHistory::SetEdge(
                            #[cfg(debug_assertions)]
                            update.from_neighbor,
                            update.side,
                            update.walls,
                            self.edge_restrictions[matching_index].restruction,
                        ));
                        self.edge_restrictions[matching_index].restruction =
                            Some(update.walls);
                    }
                },
                Err(insert_index) => {
                    has_some_updates = true;
                    self.write_to_history(CollapseHistory::SetEdge(
                        #[cfg(debug_assertions)]
                        update.from_neighbor,
                        update.side,
                        update.walls,
                        None,
                    ));
                    self.edge_restrictions.insert(
                        insert_index,
                        EdgeRestriction {
                            edge: update.side,
                            restruction: Some(update.walls)
                        },
                    )
                }
            }
        }

        // If we have not modified our restrictions, then do not send back out updates. This protects
        // against our neighbors selecting their final meshes, causing us to recognize that and selecting
        // our final mesh and sending them updates, etc.
        if has_some_updates {
            self.current_mesh = None;
            self.recompute_from_restrictions(
                log_total_restrictions,
                false,
                tiling,
                max_height,
                geom_data,
            )
        } else {
            Vec::new()
        }
    }
}

pub fn rebuild_visuals(
    mut collapse_state: ResMut<CollapseState>,
    mut events: EventReader<SimulationStateChanged>,
    mut materials: ResMut<Assets<InstancedStandardMaterial>>,
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
                collapse_state.max_height = 1; //sim_state.num_states as u32;
                collapse_state.base_tiling = sim_state.tiling.clone();
                collapse_state.dual_tiling = sim_state.tiling.get_dual();
                collapse_state.collapsed_indicies = HashSet::new();

                collapse_state.height_updates.clear();
                collapse_state.neighbor_restriction_updates.clear();

                if collapse_state.dual_tiling.kind != TilingKind::Square {
                    continue;
                }

                if collapse_state.material == Default::default() {
                    collapse_state.material = materials.add(InstancedStandardMaterial {
                        base_color: Color::INDIGO,
                        perceptual_roughness: 1.0,
                        double_sided: false,
                        cull_mode: None,
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
                                false,
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
                            collapse_state
                                .height_updates
                                .add_element(vertex, (index, state));
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
                        collapse_state
                            .height_updates
                            .add_element(vertex, (*corner, *new_value));
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
    mut debug: ResMut<DebugState>,
) {
    for _ in 0..1000 {
        if debug.breaking && !debug.step {
            return;
        }
        let was_step = debug.step;
        debug.step = false;

        // Try to take one height update out of our list of height updates.
        let index = collapse_state.height_updates.keys().next().cloned();
        if let Some(index) = index {
            if !was_step {
                for h in 0..collapse_state.max_height {
                    if debug.break_on.contains(&CollapseEntryIndex::new(index, h)) {
                        debug.breaking = true;
                        info!("Height Update: {} {:?}", index, collapse_state.height_updates.get(&index));
                        return;
                    }
                }
            }
            if let Some(updates) = collapse_state.height_updates.remove(&index) {
                for entry_height in 0..collapse_state.max_height {
                    if let Some(entity) = collapse_state
                        .position_to_entry
                        .get(&CollapseEntryIndex::new(index, entry_height))
                    {
                        if let Ok((_, entry, _, _)) = entry_query.get_mut(*entity) {
                            let mut entry: Mut<CollapseEntry> = entry;
                            let neighbor_updates = entry.vertex_set_to(
                                was_step,
                                &updates,
                                &collapse_state.dual_tiling,
                                collapse_state.max_height,
                                &geom_data,
                            );
                            collapse_state
                                .neighbor_restriction_updates
                                .extend_elements(neighbor_updates);
                        }
                    }
                }
                continue;
            }
        }

        // Try to take neighbor updates and pass them to the collapse entry
        if let Some(index) = collapse_state
            .neighbor_restriction_updates
            .keys()
            .next()
            .cloned()
        {
            if !was_step && debug.break_on.contains(&index) {
                debug.breaking = true;
                info!("Neighbor Update: {:?} {:?}", index, collapse_state.neighbor_restriction_updates.get(&index));
                return;
            }

            if let Some(updates) = collapse_state.neighbor_restriction_updates.remove(&index) {
                if let Some(entity) = collapse_state.position_to_entry.get(&index) {
                    if let Ok((_, mut entry, _, _)) = entry_query.get_mut(*entity) {
                        let neighbor_updates = entry.process_neighbor_updates(
                            was_step,
                            updates,
                            &collapse_state.dual_tiling,
                            collapse_state.max_height,
                            &geom_data,
                        );
                        collapse_state
                            .neighbor_restriction_updates
                            .extend_elements(neighbor_updates);
                    }
                }
            }
            continue;
        }

        // Now check elements that we need to select.
        let mut smallest_num = usize::MAX;
        let mut index = (0, IVec2::new(-1, -1));
        let mut entity_to_collapse = None;
        entry_query.for_each(|(entity, entry, _, _)| {
            if entry.current_mesh.is_some() {
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

        if !was_step && debug.break_on.contains(&CollapseEntryIndex::new(index.1, index.0)) {
            debug.breaking = true;
            info!("Select: {:?}", index);
            return;
        }

        let entity_to_collapse = entity_to_collapse
            .expect("Somehow we had more indicies to collapse but did not find one to");
        if let Ok((_, mut entry, mut mesh_instance, mut transform)) =
            entry_query.get_mut(entity_to_collapse)
        {
            let new_restrictions = entry.recompute_from_restrictions(
                was_step,
                true,
                &collapse_state.dual_tiling,
                collapse_state.max_height,
                &geom_data,
            );
            collapse_state
                .neighbor_restriction_updates
                .extend_elements(new_restrictions);
            if let Some(current_mesh) = entry.current_mesh {
                if let Some(new_handle) = &geom_data.mesh_handles[current_mesh.index] {
                    if new_handle.clone() != mesh_instance.mesh.clone() {
                        mesh_instance.mesh = new_handle.clone();
                    }
                }

                let new_transform = current_mesh.orientation.get_transform(
                    collapse_state
                        .dual_tiling
                        .get_tile_at_index(entry.index_in_tiling)
                        .shape
                        .get_side_count() as usize,
                );

                transform.rotation = new_transform.rotation;
                transform.scale = new_transform.scale;
            }
        }
    }
}

impl Display for CollapseHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollapseHistory::SetCorner(corner, height) => {
                write!(f, "Set Corner {} to {}", corner, height)
            }
            #[cfg(debug_assertions)]
            CollapseHistory::SetEdge(neighbor, edge, walls, old) => {
                write!(f, "{}", neighbor)?;
                write!(f, "Set Edge {} to ", edge)?;
                for wall in WallProfileIndex::from_bits(*walls) {
                    write!(f, "{},", wall.index())?;
                }
                write!(f, " from ")?;
                for wall in WallProfileIndex::from_bits(old.clone().unwrap_or(0)) {
                    write!(f, "{}", wall.index())?;
                }
                Ok(())
            }
            #[cfg(not(debug_assertions))]
            CollapseHistory::SetEdge(edge, walls, old) => {
                write!(f, "Set Edge {} to ", edge)?;
                for wall in WallProfileIndex::from_bits(*walls) {
                    write!(f, "{}", wall.index())?;
                }
                write!(f, " from ")?;
                for wall in WallProfileIndex::from_bits(old.clone().unwrap_or(0)) {
                    write!(f, "{}", wall.index())?;
                }
                Ok(())
            }
            CollapseHistory::Selected(handle, out_of) => {
                write!(f, "Selected {} from {}", handle, out_of)
            }
            CollapseHistory::DownTo(handle) => {
                write!(f, "Down to {}", handle)
            }
            CollapseHistory::Deselected(cleared_edges) => write!(
                f,
                "Deselected{}",
                if *cleared_edges {
                    " and cleared dges"
                } else {
                    ""
                }
            ),
            CollapseHistory::SendingUpdates(adjacent_index, height, walls) => {
                write!(f, "Sending Update ")?;
                for wall in WallProfileIndex::from_bits(*walls) {
                    write!(f, "{},", wall.index())?;
                }
                write!(f, " to {}@{}", adjacent_index, height)?;
                Ok(())
            }
        }
    }
}
