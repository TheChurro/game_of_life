use super::{VerticalProfile, WallProfile, orientations::GeomOrientation};


pub struct SocketProfile {
    pub bottom: Vec<VerticalProfile>,
    pub walls: Vec<WallProfile>,
    pub top: Vec<VerticalProfile>,
    pub transforms: Vec<GeomOrientation>,
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
            transforms: vec![GeomOrientation::Standard { rotations: 0 }],
        })
    }

    pub fn with_transforms(self, transforms: Vec<GeomOrientation>) -> Self {
        Self { transforms, ..self }
    }

    pub fn get_side_count(&self) -> usize {
        self.walls.len()
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

    pub fn get_wall(&self, side: usize, transform: GeomOrientation) -> WallProfile {
        self.walls[transform.get_index_in_sequence(side, self.walls.len())]
    }

    pub fn get_vertical_indicator_transform_triples(
        &self,
    ) -> Vec<(usize, usize, GeomOrientation)> {
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
    ) -> Vec<(WallProfile, GeomOrientation)> {
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