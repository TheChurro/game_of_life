
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GeomOrientation {
    Standard { rotations: usize },
    Flipped { rotations: usize },
}

impl GeomOrientation {
    pub fn get_index_in_sequence(&self, in_index: usize, max_index: usize) -> usize {
        match self {
            GeomOrientation::Standard { rotations } => (in_index + *rotations) % max_index,
            GeomOrientation::Flipped { rotations } => {
                max_index - 1 - ((in_index + *rotations) % max_index)
            }
        }
    }

    pub fn is_reversed(&self) -> bool {
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

    pub fn from_bits(bits: usize, max_rotations: usize) -> impl Iterator<Item=GeomOrientation> {
        (0..2 * max_rotations).filter_map(move |rotation| {
            let transform = if rotation < max_rotations {
                GeomOrientation::Standard { rotations: rotation }
            } else {
                GeomOrientation::Flipped { rotations: rotation - max_rotations }
            };
            if transform.to_bits() & bits != 0 {
                Some(transform)
            } else { None }
        })
    }
}