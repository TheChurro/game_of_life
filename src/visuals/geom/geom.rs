use bevy::{
    prelude::{AssetServer, Handle, Mesh, Res, ResMut},
    utils::{HashMap, HashSet},
};

use super::{socket::SocketProfile, handles::GeometryHandle, WallProfile};

pub struct GeometryStorage {
    pub mesh_handles: Vec<Option<Handle<Mesh>>>,
    pub profiles: Vec<SocketProfile>,
    pub vertical_indicator_to_geom_handle: HashMap<(usize, usize), HashSet<GeometryHandle>>,
    pub side_wall_profile_to_geom_handle: HashMap<(usize, WallProfile), HashSet<GeometryHandle>>,
}

impl GeometryStorage {
    pub fn new() -> Self {
        Self {
            mesh_handles: Vec::new(),
            profiles: Vec::new(),
            vertical_indicator_to_geom_handle: HashMap::new(),
            side_wall_profile_to_geom_handle: HashMap::new(),
        }
    }

    pub fn store(&mut self, profile: SocketProfile, mesh: Option<Handle<Mesh>>) {
        let index = self.mesh_handles.len();
        self.mesh_handles.push(mesh);

        for (bottom, top, transform) in profile.get_vertical_indicator_transform_triples() {
            if !self
                .vertical_indicator_to_geom_handle
                .contains_key(&(bottom, top))
            {
                self.vertical_indicator_to_geom_handle
                    .insert((bottom, top), HashSet::new());
            }
            if let Some(handle_set) = self
                .vertical_indicator_to_geom_handle
                .get_mut(&(bottom, top))
            {
                handle_set.insert(GeometryHandle { index, transform });
            }
        }

        for side in 0..4 {
            for (profile, transform) in profile.get_wall_profile_rotation_pairs_for_index(side) {
                if !self
                    .side_wall_profile_to_geom_handle
                    .contains_key(&(side, profile))
                {
                    self.side_wall_profile_to_geom_handle
                        .insert((side, profile), HashSet::new());
                }
                if let Some(handle_set) = self
                    .side_wall_profile_to_geom_handle
                    .get_mut(&(side, profile))
                {
                    handle_set.insert(GeometryHandle { index, transform });
                }
            }
        }

        self.profiles.push(profile);
    }
}

pub fn load_geometry(mut geom_storage: ResMut<GeometryStorage>, asset_server: Res<AssetServer>) {
    let profiles = get_rect_profiles();
    for profile in profiles {
        let mesh = asset_server.load(&profile.get_resource_location());
        geom_storage.store(profile, Some(mesh));
    }

    // Add empty space
    geom_storage.store(
        SocketProfile::new(
            "ssss".to_string(),
            vec![
                WallProfile::Empty,
                WallProfile::Empty,
                WallProfile::Empty,
                WallProfile::Empty,
            ],
            "ssss".to_string(),
        )
        .unwrap(),
        None,
    );
}

fn get_rect_profiles() -> Vec<SocketProfile> {
    use super::GeomOrientation::*;
    use WallProfile::*;
    vec![
        // Flats
        SocketProfile::new(
            "ffff".to_string(),
            vec![Bottom, Bottom, Bottom, Bottom],
            "eeee".to_string(),
        )
        .unwrap(),
        SocketProfile::new(
            "ssss".to_string(),
            vec![Top, Top, Top, Top],
            "ffff".to_string(),
        )
        .unwrap(),
        // Ramps
        // SocketProfile::new(
        //     "ffss".to_string(),
        //     vec![Bottom, Ramp, Top, Pmar],
        //     "eeff".to_string(),
        // )
        // .unwrap()
        // .with_transforms(vec![
        //     Standard { rotations: 0 },
        //     Standard { rotations: 1 },
        //     Standard { rotations: 2 },
        //     Standard { rotations: 3 },
        // ]),
        // SocketProfile::new(
        //     "fffs".to_string(),
        //     vec![Bottom, Bottom, Wall, Pmar],
        //     "eeef".to_string(),
        // )
        // .unwrap()
        // .with_transforms(vec![
        //     Standard { rotations: 0 },
        //     Standard { rotations: 1 },
        //     Standard { rotations: 2 },
        //     Standard { rotations: 3 },
        //     Flipped { rotations: 0 },
        //     Flipped { rotations: 1 },
        //     Flipped { rotations: 2 },
        //     Flipped { rotations: 3 },
        // ]),
        // SocketProfile::new(
        //     "fees".to_string(),
        //     vec![Bottom, Empty, Wall, Pmar],
        //     "eeef".to_string(),
        // )
        // .unwrap()
        // .with_transforms(vec![
        //     Standard { rotations: 0 },
        //     Standard { rotations: 1 },
        //     Standard { rotations: 2 },
        //     Standard { rotations: 3 },
        //     Flipped { rotations: 0 },
        //     Flipped { rotations: 1 },
        //     Flipped { rotations: 2 },
        //     Flipped { rotations: 3 },
        // ]),
        // SocketProfile::new(
        //     "fees".to_string(),
        //     vec![Bottom, Empty, Top, Pmar],
        //     "eeef".to_string(),
        // )
        // .unwrap()
        // .with_transforms(vec![
        //     Standard { rotations: 0 },
        //     Standard { rotations: 1 },
        //     Standard { rotations: 2 },
        //     Standard { rotations: 3 },
        //     Flipped { rotations: 0 },
        //     Flipped { rotations: 1 },
        //     Flipped { rotations: 2 },
        //     Flipped { rotations: 3 },
        // ]),
        // Corner Pillars
        SocketProfile::new(
            "fffs".to_string(),
            vec![Bottom, Bottom, Wall, Llaw],
            "eeef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        SocketProfile::new(
            "fffs".to_string(),
            vec![Bottom, Bottom, Wall, Llaw],
            "eees".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        SocketProfile::new(
            "eees".to_string(),
            vec![Empty, Empty, Wall, Llaw],
            "eeef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        SocketProfile::new(
            "eees".to_string(),
            vec![Empty, Empty, Wall, Llaw],
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
        SocketProfile::new(
            "ffss".to_string(),
            vec![Bottom, Wall, Top, Llaw],
            "eeff".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        SocketProfile::new(
            "ffss".to_string(),
            vec![Bottom, Wall, Empty, Llaw],
            "eess".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        SocketProfile::new(
            "eess".to_string(),
            vec![Empty, Wall, Top, Llaw],
            "eeff".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        SocketProfile::new(
            "eess".to_string(),
            vec![Empty, Wall, Empty, Llaw],
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
        SocketProfile::new(
            "fsfs".to_string(),
            vec![Wall, Llaw, Wall, Llaw],
            "efef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![Standard { rotations: 0 }, Standard { rotations: 1 }]),
        SocketProfile::new(
            "eses".to_string(),
            vec![Wall, Llaw, Wall, Llaw],
            "efef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![Standard { rotations: 0 }, Standard { rotations: 1 }]),
        SocketProfile::new(
            "esfs".to_string(),
            vec![Wall, Llaw, Wall, Llaw],
            "efef".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        SocketProfile::new(
            "fsfs".to_string(),
            vec![Wall, Llaw, Wall, Llaw],
            "eses".to_string(),
        )
        .unwrap()
        .with_transforms(vec![Standard { rotations: 0 }, Standard { rotations: 1 }]),
        SocketProfile::new(
            "esfs".to_string(),
            vec![Wall, Llaw, Wall, Llaw],
            "eses".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        SocketProfile::new(
            "eses".to_string(),
            vec![Wall, Llaw, Wall, Llaw],
            "eses".to_string(),
        )
        .unwrap()
        .with_transforms(vec![Standard { rotations: 0 }, Standard { rotations: 1 }]),
        //  Cut out corner
        SocketProfile::new(
            "fsss".to_string(),
            vec![Wall, Top, Top, Llaw],
            "efff".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        SocketProfile::new(
            "esss".to_string(),
            vec![Wall, Top, Top, Llaw],
            "efff".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        SocketProfile::new(
            "fsss".to_string(),
            vec![Wall, Empty, Empty, Llaw],
            "esss".to_string(),
        )
        .unwrap()
        .with_transforms(vec![
            Standard { rotations: 0 },
            Standard { rotations: 1 },
            Standard { rotations: 2 },
            Standard { rotations: 3 },
        ]),
        SocketProfile::new(
            "esss".to_string(),
            vec![Wall, Empty, Empty, Llaw],
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