use bevy::{
    prelude::{AssetServer, Handle, Mesh, Res, ResMut},
    utils::{HashMap, HashSet},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallProfile {
    Empty,
    Ramp,
    Pmar,
    Wall,
    Llaw,
    Bottom,
    Top,
}

impl WallProfile {
    // pub fn can_connect_at_level_to(self, other: WallProfile) -> bool {
    //     self == self.reverse()
    //         || (self == WallProfile::Empty && other == WallProfile::Bottom)
    //         || (self == WallProfile::Bottom && other == WallProfile::Empty)
    // }

    pub fn reverse(self) -> WallProfile {
        match self {
            WallProfile::Empty => WallProfile::Empty,
            WallProfile::Ramp => WallProfile::Pmar,
            WallProfile::Pmar => WallProfile::Ramp,
            WallProfile::Wall => WallProfile::Llaw,
            WallProfile::Llaw => WallProfile::Wall,
            WallProfile::Bottom => WallProfile::Bottom,
            WallProfile::Top => WallProfile::Top,
        }
    }

    // pub fn can_connect_below_to(self, other: WallProfile) -> bool {
    //     self == WallProfile::Bottom && other == WallProfile::Top
    // }

    pub fn label(self) -> &'static str {
        match self {
            WallProfile::Empty => "empty",
            WallProfile::Ramp => "ramp",
            WallProfile::Pmar => "pmar",
            WallProfile::Wall => "wall",
            WallProfile::Llaw => "llaw",
            WallProfile::Bottom => "bottom",
            WallProfile::Top => "top",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum VerticalProfile {
    Empty,
    Stackable,
    Full,
}

const VERTICAL_PROFILE_LEN: usize = 2;

impl VerticalProfile {
    // pub fn can_stack_on(self, other: VerticalProfile) -> bool {
    //     match (self, other) {
    //         (VerticalProfile::Empty, VerticalProfile::Stackable) => false,
    //         (VerticalProfile::Empty, _) => true,
    //         (VerticalProfile::Full, VerticalProfile::Stackable) => true,
    //         (VerticalProfile::Stackable, VerticalProfile::Stackable) => true,
    //         _ => false,
    //     }
    // }

    pub fn label(self) -> &'static str {
        match self {
            VerticalProfile::Empty => "e",
            VerticalProfile::Stackable => "s",
            VerticalProfile::Full => "f",
        }
    }

    pub fn value(self) -> usize {
        match self {
            VerticalProfile::Empty => 0,
            VerticalProfile::Stackable => 1,
            VerticalProfile::Full => 2,
        }
    }

    pub fn parse_from(pattern: String) -> Result<Vec<Self>, SocketProfileCreationError> {
        let mut sequence = Vec::new();
        for char in pattern.chars() {
            sequence.push(match char {
                's' => VerticalProfile::Stackable,
                'e' => VerticalProfile::Empty,
                'f' => VerticalProfile::Full,
                _ => return Err(SocketProfileCreationError::InvalidVerticalPattern),
            });
        }
        Ok(sequence)
    }

    pub fn compute_indicator(
        sequence: &Vec<VerticalProfile>,
        rotation: GeomTransformation,
    ) -> usize {
        let mut indicator = 0;
        for i in 0..sequence.len() {
            indicator |= sequence[rotation.get_index_in_sequence(i, sequence.len())].value()
                << i * VERTICAL_PROFILE_LEN;
        }
        indicator
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GeomTransformation {
    Standard { rotations: usize },
    Flipped { rotations: usize },
}

impl GeomTransformation {
    pub fn get_index_in_sequence(&self, in_index: usize, max_index: usize) -> usize {
        match self {
            GeomTransformation::Standard { rotations } => (in_index + *rotations) % max_index,
            GeomTransformation::Flipped { rotations } => {
                max_index - 1 - ((in_index + *rotations) % max_index)
            }
        }
    }

    pub fn is_reversed(&self) -> bool {
        match self {
            GeomTransformation::Standard { .. } => false,
            GeomTransformation::Flipped { .. } => true,
        }
    }
}

pub struct SocketProfile {
    pub bottom: Vec<VerticalProfile>,
    pub walls: Vec<WallProfile>,
    pub top: Vec<VerticalProfile>,
    pub transforms: Vec<GeomTransformation>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SocketProfileCreationError {
    MimatchedCount,
    InvalidVerticalPattern,
}

impl SocketProfile {
    pub fn new(
        bottom_pattern: String,
        walls: Vec<WallProfile>,
        top_pattern: String,
    ) -> Result<Self, SocketProfileCreationError> {
        let bottom = VerticalProfile::parse_from(bottom_pattern)?;
        let top = VerticalProfile::parse_from(top_pattern)?;
        if bottom.len() != top.len() || bottom.len() != walls.len() {
            return Err(SocketProfileCreationError::MimatchedCount);
        }
        Ok(Self {
            bottom,
            walls,
            top,
            transforms: vec![GeomTransformation::Standard { rotations: 0 }],
        })
    }

    pub fn with_transforms(self, transforms: Vec<GeomTransformation>) -> Self {
        Self { transforms, ..self }
    }

    pub fn get_resource_location(&self) -> String {
        let mut name = "rect/".to_string();
        for vertical_block in &self.bottom {
            name.push_str(vertical_block.label());
        }
        name.push('_');

        for wall in &self.walls {
            name.push_str(wall.label());
            name.push('_');
        }

        for vertical_block in &self.top {
            name.push_str(vertical_block.label());
        }

        name.push_str(".obj");
        name
    }

    pub fn get_wall(&self, side: usize, transform: GeomTransformation) -> WallProfile {
        self.walls[transform.get_index_in_sequence(side, self.walls.len())]
    }

    pub fn get_vertical_indicator_transform_triples(
        &self,
    ) -> Vec<(usize, usize, GeomTransformation)> {
        self.transforms
            .iter()
            .map(|transform| {
                (
                    VerticalProfile::compute_indicator(&self.bottom, *transform),
                    VerticalProfile::compute_indicator(&self.top, *transform),
                    *transform,
                )
            })
            .collect()
    }

    pub fn get_wall_profile_rotation_pairs_for_index(
        &self,
        index: usize,
    ) -> Vec<(WallProfile, GeomTransformation)> {
        let mut out = Vec::new();
        for transform in &self.transforms {
            let profile = self.walls[transform.get_index_in_sequence(index, self.walls.len())];
            out.push((
                if transform.is_reversed() {
                    profile.reverse()
                } else {
                    profile
                },
                *transform,
            ))
        }
        out
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GeometryHandle {
    pub index: usize,
    pub transform: GeomTransformation,
}

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
    use GeomTransformation::*;
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
