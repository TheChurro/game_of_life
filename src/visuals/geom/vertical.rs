use super::{orientations::GeomOrientation, socket::SocketProfileCreationError};

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

    pub fn compute_indicator(sequence: &Vec<VerticalProfile>, rotation: GeomOrientation) -> usize {
        let mut indicator = 0;
        for i in 0..sequence.len() {
            indicator |= sequence[rotation.get_index_in_sequence(i, sequence.len(), true)].value()
                << i * VERTICAL_PROFILE_LEN;
        }
        indicator
    }
}
