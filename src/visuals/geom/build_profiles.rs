use std::{cmp::Ordering};

use bevy::{
    math::{Vec2, Vec3, Vec3Swizzles},
    prelude::Mesh,
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    utils::HashMap,
};

use super::GeomOrientation;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct WallProfileIndex(u8);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LayerProfileIndex(u8);

impl WallProfileIndex {
    pub fn new(index: usize) -> Self {
        Self(index as u8)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }

    pub fn to_bits(self) -> u128 {
        1 << self.0 as u128
    }

    pub fn from_bits(mut bits: u128) -> Vec<WallProfileIndex> {
        let mut profiles = Vec::new();
        let mut profile = 0;
        while bits > 0 {
            if (bits & 1) != 0 {
                profiles.push(WallProfileIndex(profile));
            }
            bits >>= 1;
            profile += 1;
        }

        profiles
    }
}

impl LayerProfileIndex {
    pub fn new(index: usize) -> Self {
        Self(index as u8)
    }

    #[allow(unused)]
    pub fn to_bits(self) -> u128 {
        1 << self.0 as u128
    }

    #[allow(unused)]
    pub fn from_bits(mut bits: u128) -> Vec<LayerProfileIndex> {
        let mut profiles = Vec::new();
        let mut profile = 0;
        while bits > 0 {
            if (bits & 1) != 0 {
                profiles.push(LayerProfileIndex(profile));
            }
            bits >>= 1;
            profile += 1;
        }

        profiles
    }
}

#[derive(Clone)]
pub struct ProfileDefinition {
    pub verticies: Vec<Vec2>,
    pub edges: Vec<(usize, usize)>,
}

trait HasProfileDefinition {
    fn get_profile_definition(&self) -> &ProfileDefinition;
}

#[derive(Clone)]
pub struct WallProfileDefinition {
    pub definition: ProfileDefinition,
    pub reverse_profile: WallProfileIndex,
}

impl HasProfileDefinition for WallProfileDefinition {
    fn get_profile_definition(&self) -> &ProfileDefinition {
        &self.definition
    }
}

#[derive(Clone)]
pub struct LayerProfileDefinition {
    pub definition: ProfileDefinition,
    pub side_count: usize,
    pub orientation_map: HashMap<GeomOrientation, LayerProfileIndex>,
}

impl HasProfileDefinition for LayerProfileDefinition {
    fn get_profile_definition(&self) -> &ProfileDefinition {
        &self.definition
    }
}

#[derive(Clone)]
pub struct MeshProfile {
    pub sides: usize,
    pub walls: Vec<WallProfileIndex>,
    pub top: LayerProfileIndex,
    pub bottom: LayerProfileIndex,
    pub orientations: Vec<GeomOrientation>,
}

const TOLERANCE: f32 = 0.0001;

fn compute_face_profile(
    mesh: &Mesh,
    face_normal: Vec3,
    distance_to_normal: f32,
) -> ProfileDefinition {
    let mut verticies = Vec::with_capacity(0);
    let mut edges = Vec::new();

    // Ensure our face_normal has length 1!
    let face_normal = face_normal.normalize();
    // Determine the coordinate space for our verticies perpendicular to the face
    let (axis_w, axis_h) = if face_normal.dot(Vec3::Y) < TOLERANCE {
        (face_normal.cross(Vec3::Y), Vec3::Y)
    } else {
        let w = face_normal.cross(Vec3::Z);
        (w, w.cross(face_normal))
    };

    assert!(mesh.primitive_topology() == PrimitiveTopology::TriangleList);

    if let (Some(VertexAttributeValues::Float32x3(mesh_verticies)), Some(mesh_faces)) =
        (mesh.attribute(Mesh::ATTRIBUTE_POSITION), mesh.indices())
    {
        // Iterate over verticies in our mesh, determine if they lie upon the passed in face
        // and if so calculate their position on the face and store their index in the mesh
        let mut vertex_scratch = Vec::<(Vec2, usize)>::new();
        let mut equivalent_mesh_indexes = HashMap::new();
        for (index, vertex) in mesh_verticies.iter().enumerate() {
            let vertex = Vec3::new(vertex[0], vertex[1], vertex[2]);
            let distance = vertex.dot(face_normal);
            if (distance - distance_to_normal).abs() < TOLERANCE {
                let w_pos = vertex.dot(axis_w);
                let h_pos = vertex.dot(axis_h);
                let face_vertex = Vec2::new(w_pos, h_pos);
                match vertex_scratch.binary_search_by(|(vertex, _)| {
                    vertex.partial_cmp(&face_vertex).unwrap_or(Ordering::Less)
                }) {
                    Ok(matched_index) => {
                        equivalent_mesh_indexes.insert(index, vertex_scratch[matched_index].1);
                    }
                    Err(insert_index) => {
                        vertex_scratch.insert(insert_index, (face_vertex, index));
                    }
                }
            }
        }

        // Convert our (vertex, index) pairs into a vector and map from mesh index to face index
        // for use in the face profile and edge extraction respectively
        let mut index_to_vertex = HashMap::new();
        verticies = Vec::with_capacity(vertex_scratch.len());
        for (index, (vertex, mesh_index)) in vertex_scratch.into_iter().enumerate() {
            verticies.push(vertex);
            index_to_vertex.insert(mesh_index, index);
        }
        for (duplicated_index, inserted_index) in equivalent_mesh_indexes {
            let face_index = index_to_vertex[&inserted_index];
            index_to_vertex.insert(duplicated_index, face_index);
        }

        fn get_face(indicies: &Indices, face: usize) -> Option<[usize; 3]> {
            let base = 3 * face;
            match indicies {
                Indices::U16(indicies) => {
                    if base + 2 < indicies.len() {
                        Some([
                            indicies[base] as usize,
                            indicies[base + 1] as usize,
                            indicies[base + 2] as usize,
                        ])
                    } else {
                        None
                    }
                }
                Indices::U32(indicies) => {
                    if base + 2 < indicies.len() {
                        Some([
                            indicies[base] as usize,
                            indicies[base + 1] as usize,
                            indicies[base + 2] as usize,
                        ])
                    } else {
                        None
                    }
                }
            }
        }

        fn add_edge(edges: &mut Vec<(usize, usize)>, edge: (usize, usize)) {
            let edge = if edge.0 <= edge.1 {
                edge
            } else {
                (edge.1, edge.0)
            };

            match edges.binary_search(&edge) {
                Err(insert_index) => {
                    edges.insert(insert_index, edge);
                }
                _ => {}
            }
        }

        // Iterate over the edges and add the edges along this face to the profile
        for face in 0..mesh_faces.len() / 3 {
            if let Some([a, b, c]) = get_face(mesh_faces, face) {
                match (
                    index_to_vertex.get(&a),
                    index_to_vertex.get(&b),
                    index_to_vertex.get(&c),
                ) {
                    (None, Some(e0), Some(e1)) => {
                        add_edge(&mut edges, (*e0, *e1));
                    }
                    (Some(e1), None, Some(e0)) => {
                        add_edge(&mut edges, (*e0, *e1));
                    }
                    (Some(e0), Some(e1), None) => {
                        add_edge(&mut edges, (*e0, *e1));
                    }
                    (Some(e0), Some(e1), Some(e2)) => {
                        add_edge(&mut edges, (*e0, *e1));
                        add_edge(&mut edges, (*e1, *e2));
                        add_edge(&mut edges, (*e2, *e0));
                    }
                    _ => {}
                }
            }
        }
    }

    ProfileDefinition {
        verticies,
        edges,
    }
}

fn apply_orientation(
    profile: &ProfileDefinition,
    orientation: GeomOrientation,
    max_sides: usize,
) -> ProfileDefinition {
    let transform = orientation.get_transform(max_sides);
    let mut vertex_scratch = profile
        .verticies
        .iter()
        .map(|vertex| (transform * Vec3::new(vertex.y, 0.0, vertex.x)).zx())
        .enumerate()
        .collect::<Vec<_>>();
    vertex_scratch.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Less));

    let mut verticies = Vec::with_capacity(vertex_scratch.len());
    let mut old_vertex_to_new = vec![0; vertex_scratch.len()];

    for (new_index, (old_index, vertex)) in vertex_scratch.into_iter().enumerate() {
        verticies.push(vertex);
        old_vertex_to_new[old_index] = new_index;
    }

    let mut edges = Vec::with_capacity(profile.edges.len());
    for edge in &profile.edges {
        let new_edge =  if old_vertex_to_new[edge.0] <= old_vertex_to_new[edge.1] {
            (old_vertex_to_new[edge.0], old_vertex_to_new[edge.1])
        } else {
            (old_vertex_to_new[edge.1], old_vertex_to_new[edge.0])
        };

        match edges.binary_search(&new_edge) {
            Err(index) => edges.insert(index, new_edge),
            _ => (),
        }
    }

    ProfileDefinition {
        verticies,
        edges,
    }
}

fn are_same_profile(a: &ProfileDefinition, b: &ProfileDefinition) -> bool {
    if a.verticies.len() != b.verticies.len() {
        return false;
    }
    if a.edges.len() != b.edges.len() {
        return false;
    }

    if a.verticies
        .iter()
        .zip(&b.verticies)
        .any(|(a, b)| (*a - *b).length_squared() > TOLERANCE)
    {
        return false;
    }

    a.edges == b.edges
}

fn get_matching_profile<T: HasProfileDefinition>(
    new_profile: &ProfileDefinition,
    profiles: &Vec<T>,
) -> Option<usize> {
    for (i, profile) in profiles.iter().enumerate() {
        if are_same_profile(new_profile, profile.get_profile_definition()) {
            return Some(i);
        }
    }

    None
}

impl HasProfileDefinition for (GeomOrientation, LayerProfileDefinition) {
    fn get_profile_definition(&self) -> &ProfileDefinition {
        self.1.get_profile_definition()
    }
}

fn get_or_insert_layer_profiles(new_profile: ProfileDefinition, orientations: &Vec<GeomOrientation>, max_sides: usize, profiles: &mut Vec<LayerProfileDefinition>) -> LayerProfileIndex {
    let mut index_for_orientation = Vec::new();
    for orientation in orientations {
        index_for_orientation.push((
            *orientation,
            match get_matching_profile(&new_profile, profiles) {
                Some(index) => index,
                None => {
                    let new_index = LayerProfileIndex(profiles.len() as u8);
                    let mut orientation_map = HashMap::new();
                    orientation_map.insert(GeomOrientation::Standard { rotations: 0 }, new_index);
                    profiles.push(LayerProfileDefinition {
                        definition: apply_orientation(&new_profile, *orientation, max_sides),
                        side_count: max_sides,
                        orientation_map
                    });
                    new_index.0 as usize
                }
            }
        ));
    }

    for (orientation_0, index_0) in &index_for_orientation {
        for (orientation_1, index_1) in &index_for_orientation {
            profiles[*index_0].orientation_map.insert(
                orientation_0.inverse(max_sides).compose(*orientation_1, max_sides),
                LayerProfileIndex(*index_1 as u8)
            );
        }
    }
    
    LayerProfileIndex(index_for_orientation[0].1 as u8)
}

pub fn generate_profiles_for_mesh(
    mesh: &Mesh,
    orientations: Vec<GeomOrientation>,
    distance_to_sides: f32,
    num_sides: usize,
    wall_profiles: &mut Vec<WallProfileDefinition>,
    layer_profiles: &mut Vec<LayerProfileDefinition>,
) -> MeshProfile {
    let mut walls = Vec::with_capacity(num_sides);

    // For each side of the mesh...
    for side in 0..num_sides {
        // Compute the profile for the corresponding face
        let angle = std::f32::consts::FRAC_PI_2 - std::f32::consts::TAU * side as f32 / num_sides as f32;
        let axis = Vec3::new(angle.cos(), 0.0, angle.sin());
        let profile = compute_face_profile(mesh, axis, distance_to_sides);

        // Determine if we have already registered that profile, and if so push the matching
        // profile to our mesh's face -> profile list. If not, we will insert a new profile
        // into the existing face profiles list and the reverse of that profile if the profile
        // is not symmetrical. We will append the index of the computed profile to our
        // face -> profile list.
        match get_matching_profile(&profile, wall_profiles) {
            Some(existing_profile) => walls.push(WallProfileIndex(existing_profile as u8)),
            None => {
                // Push now when we know that the next profile to be added (the profile
                // we computed for this face) is the size of the existing profiles.
                walls.push(WallProfileIndex(wall_profiles.len() as u8));

                let reversed = apply_orientation(
                    &profile,
                    GeomOrientation::Flipped { rotations: 0 },
                    num_sides,
                );
                if are_same_profile(&profile, &reversed) {
                    wall_profiles.push(WallProfileDefinition {
                        definition: profile,
                        reverse_profile: WallProfileIndex(wall_profiles.len() as u8),
                    });
                } else {
                    wall_profiles.push(WallProfileDefinition {
                        definition: profile,
                        reverse_profile: WallProfileIndex(wall_profiles.len() as u8 + 1),
                    });
                    wall_profiles.push(WallProfileDefinition {
                        definition: reversed,
                        reverse_profile: WallProfileIndex(wall_profiles.len() as u8 - 1),
                    });
                }
            }
        }
    }

    let bottom_profile = compute_face_profile(mesh, -Vec3::Y, 0.0);
    let bottom = get_or_insert_layer_profiles(
        bottom_profile,
        &orientations,
        num_sides,
        layer_profiles
    );
    let top_profile = compute_face_profile(mesh, Vec3::Y, 1.0);
    let top = get_or_insert_layer_profiles(
        top_profile,
        &orientations,
        num_sides,
        layer_profiles
    );

    MeshProfile { sides: num_sides, walls, bottom, top, orientations }
}