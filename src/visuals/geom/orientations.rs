use bevy::{
    math::{Quat, Vec3},
    prelude::Transform,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GeomOrientation {
    Standard { rotations: usize },
    Flipped { rotations: usize },
}

impl Default for GeomOrientation {
    fn default() -> Self {
        Self::Standard { rotations: 0 }
    }
}

impl GeomOrientation {
    pub fn get_index_in_sequence(
        &self,
        in_index: usize,
        max_index: usize,
        is_corner: bool,
    ) -> usize {
        match self {
            GeomOrientation::Standard { rotations } => (in_index + *rotations) % max_index,
            GeomOrientation::Flipped { rotations } => {
                (4 * max_index - if is_corner { 1 } else { 2 } - in_index + *rotations)
                    .rem_euclid(max_index)
            }
        }
    }

    pub const fn is_reversed(&self) -> bool {
        match self {
            GeomOrientation::Standard { .. } => false,
            GeomOrientation::Flipped { .. } => true,
        }
    }

    pub const fn to_bits(self) -> usize {
        match self {
            GeomOrientation::Standard { rotations } => 1 << rotations,
            GeomOrientation::Flipped { rotations } => 1 << (usize::BITS as usize - 1 - rotations),
        }
    }

    pub fn from_bits(bits: usize, max_rotations: usize) -> impl Iterator<Item = GeomOrientation> {
        (0..2 * max_rotations).filter_map(move |rotation| {
            let transform = if rotation < max_rotations {
                GeomOrientation::Standard {
                    rotations: rotation,
                }
            } else {
                GeomOrientation::Flipped {
                    rotations: rotation - max_rotations,
                }
            };
            if transform.to_bits() & bits != 0 {
                Some(transform)
            } else {
                None
            }
        })
    }

    pub fn get_transform(&self, max_sides: usize) -> Transform {
        match self {
            GeomOrientation::Standard { rotations } => {
                Transform::from_rotation(Quat::from_rotation_y(
                    -std::f32::consts::TAU * *rotations as f32 / max_sides as f32,
                ))
            }
            GeomOrientation::Flipped { rotations } => Transform::from_rotation(
                Quat::from_rotation_y(std::f32::consts::TAU * *rotations as f32 / max_sides as f32),
            )
            .with_scale(Vec3::new(1.0, 1.0, -1.0)),
        }
    }

    pub fn inverse(&self, max_sides: usize) -> GeomOrientation {
        match self {
            GeomOrientation::Standard { rotations } => GeomOrientation::Standard { rotations: (max_sides - rotations) % max_sides },
            GeomOrientation::Flipped { rotations } => GeomOrientation::Flipped { rotations: (max_sides - rotations) % max_sides },
        }
    }

    pub fn compose(&self, other: GeomOrientation, max_sides: usize) -> GeomOrientation {
        match (self, other) {
            (GeomOrientation::Standard { rotations: rot1 }, GeomOrientation::Standard { rotations: rot2 }) => 
                GeomOrientation::Standard { rotations: (*rot1 + rot2) % max_sides },
            (GeomOrientation::Standard { rotations: rot1 }, GeomOrientation::Flipped { rotations: rot2 }) => 
                GeomOrientation::Flipped { rotations: (*rot1 + rot2) % max_sides },
            (GeomOrientation::Flipped { rotations: rot1 }, GeomOrientation::Flipped { rotations: rot2 }) => 
                GeomOrientation::Standard { rotations: (*rot1 + rot2) % max_sides },
            (GeomOrientation::Flipped { rotations: rot1 }, GeomOrientation::Standard { rotations: rot2 }) => 
                GeomOrientation::Flipped { rotations: (*rot1 + rot2) % max_sides },
        }
    }
}
