use bevy::{
    prelude::{AssetServer, Assets, Handle, Local, Mesh, Res, ResMut, Color, Visibility, Component, Query, KeyCode, With, EventWriter, info},
    render::mesh::Indices,
    utils::HashMap, asset::LoadState, pbr::StandardMaterial, input::Input,
};

use crate::{ui::InputState, menus::CommandEvent};

use super::{
    build_profiles::{generate_profiles_for_mesh, WallProfileDefinition, MeshProfile, WallProfileIndex, LayerProfileDefinition},
    handles::{GeometryHandle, GeometryHandleSet}, VerticalProfile, vertical::VerticalProfileParseError, GeomOrientation,
};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct GeometryStorageVerticalKey {
    pub side_count: usize,
    pub bottom_profile: usize,
    pub top_profile: usize,
}

impl GeometryStorageVerticalKey {
    pub const fn new(side_count: usize, bottom: usize, top: usize) -> Self {
        Self {
            side_count,
            bottom_profile: bottom,
            top_profile: top,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct GeometryStorageWallKey {
    pub side_count: usize,
    pub side: usize,
    pub profile: WallProfileIndex,
}

impl GeometryStorageWallKey {
    pub const fn new(side_count: usize, side: usize, profile: WallProfileIndex) -> Self {
        Self {
            side_count,
            side,
            profile,
        }
    }
}

pub struct GeometryStorage {
    pub mesh_handles: Vec<Option<Handle<Mesh>>>,
    pub profiles: Vec<MeshProfile>,
    pub vertical_indicator_to_geom_handle: HashMap<GeometryStorageVerticalKey, GeometryHandleSet>,
    pub side_wall_profile_to_geom_handle: HashMap<GeometryStorageWallKey, GeometryHandleSet>,
    pub profile_2d_meshes: Vec<Handle<Mesh>>,
    pub wall_profiles: Vec<WallProfileDefinition>,
    pub layer_profiles: Vec<LayerProfileDefinition>,

    pub base_material: Handle<StandardMaterial>,
    pub side_materials: Vec<Handle<StandardMaterial>>,
}

impl GeometryStorage {
    pub fn new() -> Self {
        Self {
            mesh_handles: Vec::new(),
            profiles: Vec::new(),
            vertical_indicator_to_geom_handle: HashMap::new(),
            side_wall_profile_to_geom_handle: HashMap::new(),
            profile_2d_meshes: Vec::new(),
            wall_profiles: Vec::new(),
            layer_profiles: Vec::new(),
            base_material: Handle::default(),
            side_materials: Vec::new(),
        }
    }

    pub fn get_wall(&self, profile: &MeshProfile, side: usize, orientation: &GeomOrientation) -> WallProfileIndex {
        let wall =  profile.walls[orientation.get_index_in_sequence(side, profile.sides, false)];
        if orientation.is_reversed() {
            self.wall_profiles[wall.index()].reverse_profile
        } else {
            wall
        }
    }

    pub fn store(&mut self, profile: MeshProfile, top_descriptor: &Vec<VerticalProfile>, bottom_descriptor: &Vec<VerticalProfile>, mesh: Option<Handle<Mesh>>) {
        let index = self.mesh_handles.len();
        self.mesh_handles.push(mesh);
        let profile_side_count = profile.sides;

        for orientation in &profile.orientations {
            let top = VerticalProfile::compute_indicator(&top_descriptor, *orientation);
            let bottom = VerticalProfile::compute_indicator(&bottom_descriptor, *orientation);
            let key = GeometryStorageVerticalKey::new(profile_side_count, bottom, top);
            if !self.vertical_indicator_to_geom_handle.contains_key(&key) {
                self.vertical_indicator_to_geom_handle
                    .insert(key, GeometryHandleSet::new(profile_side_count));
            }
            if let Some(handle_set) = self.vertical_indicator_to_geom_handle.get_mut(&key) {
                handle_set.insert(GeometryHandle {
                    index,
                    orientation: *orientation,
                });
            }

            for side in 0..profile_side_count {
                let wall_profile_index = self.get_wall(&profile, side, orientation);
                let key = GeometryStorageWallKey::new(profile_side_count, side, wall_profile_index);
                if !self.side_wall_profile_to_geom_handle.contains_key(&key) {
                    self.side_wall_profile_to_geom_handle
                        .insert(key, GeometryHandleSet::new(profile_side_count));
                }
                if let Some(handle_set) = self.side_wall_profile_to_geom_handle.get_mut(&key) {
                    handle_set.insert(GeometryHandle {
                        index,
                        orientation: *orientation,
                    });
                }
            }
        }

        self.profiles.push(profile);
    }

    pub fn get_vertical_matching(
        &self,
        side_count: usize,
        bottom: usize,
        top: usize,
    ) -> GeometryHandleSet {
        if let Some(set) = self
            .vertical_indicator_to_geom_handle
            .get(&GeometryStorageVerticalKey::new(side_count, bottom, top))
        {
            set.clone()
        } else {
            GeometryHandleSet::new(side_count)
        }
    }

    pub fn get_wall_union(
        &self,
        side_count: usize,
        side: usize,
        wall_bits: u128,
    ) -> GeometryHandleSet {
        GeometryHandleSet::union(
            WallProfileIndex::from_bits(wall_bits)
                .iter()
                .filter_map(|profile| {
                    self.side_wall_profile_to_geom_handle
                        .get(&GeometryStorageWallKey::new(side_count, side, *profile))
                }),
        )
    }

    pub fn get_walls_in_set(&self, set: &GeometryHandleSet) -> Vec<u128> {
        let mut walls = vec![0; set.get_max_rotations()];
        for handle in set {
            if let Some(profile) = self.profiles.get(handle.index) {
                for side in 0..walls.len() {
                    let wall_in_mesh = handle.orientation.get_index_in_sequence(side, profile.sides, false);
                    let mut wall_profile_index = profile.walls[wall_in_mesh];
                    if handle.orientation.is_reversed() {
                        wall_profile_index = self.wall_profiles[wall_profile_index.index()].reverse_profile;
                    }
                    walls[side] |= wall_profile_index.to_bits();
                }
            }
        }
        walls
    }
}

pub fn load_geometry(mut geom_data: ResMut<GeometryStorage>, asset_server: Res<AssetServer>) {
    // Load the mesh for every profile we have
    let profiles = get_rect_profiles();
    for profile in profiles {
        let resource_location = profile.get_resource_location();
        geom_data.mesh_handles.push(Some(asset_server.load::<Mesh, _>(&resource_location)));
    }
}

struct ObjectProfile {
    top: Vec<VerticalProfile>,
    bottom: Vec<VerticalProfile>,
    edge_labels: Vec<String>,
    transforms: Vec<GeomOrientation>,
}

impl ObjectProfile {
    fn new(bottom: String, labels: Vec<&str>, top: String) -> Result<Self, VerticalProfileParseError> {
        Ok(ObjectProfile {
            top: VerticalProfile::parse_from(top)?,
            bottom: VerticalProfile::parse_from(bottom)?,
            edge_labels: labels.into_iter().map(|x| x.to_string()).collect(),
            transforms: vec!(GeomOrientation::Standard { rotations: 0 }),
        })
    }

    fn with_transforms(self, transforms: Vec<GeomOrientation>) -> Self {
        Self {
            transforms,
            ..self
        }
    }

    fn get_resource_location(&self) -> String {
        let mut data = String::from("rect/");
        for p in &self.bottom {
            data.push_str(p.label());
        }
        data.push('_');

        for label in &self.edge_labels {
            data.push_str(&label);
            data.push('_');
        }
        
        for p in &self.top {
            data.push_str(p.label());
        }
        data.push_str(".obj");
        data
    }
}

#[derive(Component)]
pub struct DebugGeomDisplay;

pub fn log_geometry(
    mut geom_storage: ResMut<GeometryStorage>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut has_extracted: Local<bool>,
    mut colors: ResMut<Assets<StandardMaterial>>,
    mut events: EventWriter<CommandEvent>,
) {
    if !*has_extracted {
        events.send(CommandEvent("n w 00 empty".to_string()));
        events.send(CommandEvent("n w 01 floor".to_string()));
        events.send(CommandEvent("n w 02 ciel".to_string()));
        events.send(CommandEvent("n w 03 ramp".to_string()));
        events.send(CommandEvent("n w 04 pmar".to_string()));
        events.send(CommandEvent("n w 05 wall".to_string()));
        events.send(CommandEvent("n w 06 llaw".to_string()));

        for handle in &geom_storage.mesh_handles {
            if let Some(handle) = handle {
                if asset_server.get_load_state(handle) == LoadState::Loading {
                    return;
                }
            }
        }
        let mut tmp_handles = Vec::new();
        std::mem::swap(&mut tmp_handles, &mut geom_storage.mesh_handles);

        let mut empty_mesh = Mesh::new(bevy::render::mesh::PrimitiveTopology::TriangleList);
        empty_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
        empty_mesh.set_indices(Some(Indices::U16(Vec::new())));
        let empty_mesh_profile = {
            let &mut GeometryStorage {
                ref mut wall_profiles,
                ref mut layer_profiles,
                ..
            } = geom_storage.as_mut();

            generate_profiles_for_mesh(
                &empty_mesh,
                vec![GeomOrientation::Standard { rotations: 0 }],
                0.0,
                4,
                wall_profiles,
                layer_profiles
            )
        };
        let all_stackable = VerticalProfile::parse_from("ssss".to_string()).unwrap();
        let all_empty = VerticalProfile::parse_from("eeee".to_string()).unwrap();
        geom_storage.store(
            empty_mesh_profile.clone(),
            &all_stackable,
            &all_stackable,
            None
        );
        geom_storage.store(
            empty_mesh_profile,
            &all_empty,
            &all_empty,
            None
        );

        for profile in get_rect_profiles() {
            let resource_location = profile.get_resource_location();
            let mesh_handle: Handle<Mesh> = asset_server.get_handle(&resource_location);


            if let Some(mesh) = meshes.get(&mesh_handle) {
                let mesh_profile = {
                    let &mut GeometryStorage {
                        ref mut wall_profiles,
                        ref mut layer_profiles,
                        ..
                    } = geom_storage.as_mut();

                    generate_profiles_for_mesh(
                        mesh,
                        profile.transforms,
                        0.5,
                        4,
                        wall_profiles,
                        layer_profiles,
                    )
                };
                geom_storage.store(
                    mesh_profile,
                    &profile.top,
                    &profile.bottom,
                    Some(mesh_handle.clone())
                );
            }
        }

        {
            let &mut GeometryStorage {
                ref wall_profiles,
                ref mut profile_2d_meshes,
                ..
            } = geom_storage.as_mut();

            for profile in wall_profiles {
                info!("NEW PROFILE");
                let mut profile_mesh = Mesh::new(bevy::render::mesh::PrimitiveTopology::TriangleList);
                let mut verticies = Vec::with_capacity(profile.definition.verticies.len() * 2);
                let normals = vec![[0.0, 0.0, 1.0]; profile.definition.verticies.len() * 2];
                let uvs = vec![[0.0, 0.0]; profile.definition.verticies.len() * 2];
                for vertex in &profile.definition.verticies {
                    info!("  V: {}", vertex);
                    verticies.push([vertex.x - 0.05, vertex.y - 0.05, 0.0]);
                    verticies.push([vertex.x + 0.05, vertex.y + 0.05, 0.0]);
                }
                profile_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verticies);
                profile_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                profile_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                let mut indicies = Vec::new();
                for edge in &profile.definition.edges {
                    indicies.push(2 * edge.0 as u32);
                    indicies.push(2 * edge.1 as u32);
                    indicies.push(1 + 2 * edge.0 as u32);
                    indicies.push(2 * edge.1 as u32);
                    indicies.push(1 + 2 * edge.1 as u32);
                    indicies.push(1 + 2 * edge.0 as u32);
                }
                profile_mesh.set_indices(Some(Indices::U32(indicies)));
                let mesh = meshes.add(profile_mesh);
                profile_2d_meshes.push(mesh);
            }
        }

        let base_color = colors.add(StandardMaterial {
            cull_mode: None,
            ..Color::WHITE.into()
        });
        let num_walls = geom_storage.wall_profiles.len();
        let side_colors = (0..num_walls).into_iter().map(|index| colors.add(StandardMaterial {
            cull_mode: None,
            unlit: true,
            double_sided: true,
            ..Color::hsl(360.0 * index as f32 / num_walls as f32, 1.0, 0.5).into()
        })).collect::<Vec<_>>();

        geom_storage.base_material = base_color;
        geom_storage.side_materials = side_colors;

        *has_extracted = true;
    }
}

pub fn geometry_input(
    input_state: Res<InputState>,
    keyboard: Res<Input<KeyCode>>,
    mut query: Query<&mut Visibility, With<DebugGeomDisplay>>,
) {
    if !input_state.has_selection() && keyboard.just_pressed(KeyCode::D) {
        for mut visibility in query.iter_mut() {
            visibility.is_visible = !visibility.is_visible;
        }
    }
}

fn get_rect_profiles() -> Vec<ObjectProfile> {
    use super::GeomOrientation::*;
    vec![
        // Flats
        ObjectProfile::new(
            "ffff".to_string(),
            vec!["bottom", "bottom", "bottom", "bottom"],
            "eeee".to_string(),
        )
        .unwrap(),
        ObjectProfile::new(
            "ssss".to_string(),
            vec!["top", "top", "top", "top"],
            "ffff".to_string(),
        )
        .unwrap(),
        // Ramps
        ObjectProfile::new(
            "ffss".to_string(),
            vec!["bottom", "ramp", "top", "pmar"],
            "eeff".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "fffs".to_string(),
            vec!["bottom", "bottom", "wall", "pmar"],
            "eeef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
            Flipped { rotations: 0 },
            Flipped { rotations: 1 },
            Flipped { rotations: 2 },
            Flipped { rotations: 3 },
        ]),
        ObjectProfile::new(
            "fffs".to_string(),
            vec!["bottom", "bottom", "wall", "pmar", "2"],
            "eeef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
            Flipped { rotations: 0 },
            Flipped { rotations: 1 },
            Flipped { rotations: 2 },
            Flipped { rotations: 3 },
        ]),
        ObjectProfile::new(
            "fees".to_string(),
            vec!["bottom", "empty", "wall", "pmar"],
            "eeef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
            Flipped { rotations: 0 },
            Flipped { rotations: 1 },
            Flipped { rotations: 2 },
            Flipped { rotations: 3 },
        ]),
        ObjectProfile::new(
            "fees".to_string(),
            vec!["bottom", "empty", "top", "pmar"],
            "eeef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
            Flipped { rotations: 0 },
            Flipped { rotations: 1 },
            Flipped { rotations: 2 },
            Flipped { rotations: 3 },
        ]),
        // Corner Pillars
        ObjectProfile::new(
            "fffs".to_string(),
            vec!["bottom", "bottom", "wall", "llaw"],
            "eeef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "fffs".to_string(),
            vec!["bottom", "bottom", "wall", "llaw"],
            "eees".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "eees".to_string(),
            vec!["empty", "empty", "wall", "llaw"],
            "eeef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "eees".to_string(),
            vec!["empty", "empty", "wall", "llaw"],
            "eees".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        // Center hard raises
        ObjectProfile::new(
            "ffss".to_string(),
            vec!["bottom", "wall", "top", "llaw"],
            "eeff".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "ffss".to_string(),
            vec!["bottom", "wall", "empty", "llaw"],
            "eess".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "eess".to_string(),
            vec!["empty", "wall", "top", "llaw"],
            "eeff".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "eess".to_string(),
            vec!["empty", "wall", "empty", "llaw"],
            "eess".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        // Horizontal bars
        ObjectProfile::new(
            "fsfs".to_string(),
            vec!["wall", "llaw", "wall", "llaw"],
            "efef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![Standard { rotations: 0 }, Standard { rotations: 1 }]),
        ObjectProfile::new(
            "eses".to_string(),
            vec!["wall", "llaw", "wall", "llaw"],
            "efef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![Standard { rotations: 0 }, Standard { rotations: 1 }]),
        ObjectProfile::new(
            "esfs".to_string(),
            vec!["wall", "llaw", "wall", "llaw"],
            "efef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "fsfs".to_string(),
            vec!["wall", "llaw", "wall", "llaw"],
            "eses".to_string(),
        )
        .unwrap()
        .with_transforms(vec![Standard { rotations: 0 }, Standard { rotations: 1 }]),
        ObjectProfile::new(
            "esfs".to_string(),
            vec!["wall", "llaw", "wall", "llaw"],
            "eses".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "eses".to_string(),
            vec!["wall", "llaw", "wall", "llaw"],
            "eses".to_string(),
        )
        .unwrap()
        .with_transforms(vec![Standard { rotations: 0 }, Standard { rotations: 1 }]),
        //  Cut out corner
        ObjectProfile::new(
            "fsss".to_string(),
            vec!["wall", "top", "top", "llaw"],
            "efff".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "esss".to_string(),
            vec!["wall", "top", "top", "llaw"],
            "efff".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "fsss".to_string(),
            vec!["wall", "empty", "empty", "llaw"],
            "esss".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        ObjectProfile::new(
            "esss".to_string(),
            vec!["wall", "empty", "empty", "llaw"],
            "esss".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
    ]
}
