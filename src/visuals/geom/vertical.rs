use super::{orientations::GeomOrientation};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum VerticalProfile {
    Empty,
    Stackable,
    Full,
}

#[derive(Clone, Copy, Debug)]
pub enum VerticalProfileParseError {
    InvalidVerticalPattern
}

const VERTICAL_PROFILE_LEN: usize = 2;

impl VerticalProfile {

    pub fn label(self) -> &'static str {
        match self {
            VerticalProfile::Empty => "e",
            VerticalProfile::Stackable => "s",
            VerticalProfile::Full => "f",
        }
    }

    pub fn label_string(profiles: &Vec<VerticalProfile>) -> String {
        let mut label = String::new();
        for p in profiles {
            label.push_str(p.label());
        }
        label
    }

    pub fn from_bits(mut indicator: usize) -> Vec<VerticalProfile> {
        let mut profiles = Vec::new();
        while indicator > 0 {
            match indicator & 3 {
                1 => profiles.push(VerticalProfile::Empty),
                2 => profiles.push(VerticalProfile::Stackable),
                3 => profiles.push(VerticalProfile::Full),
                _ => (),
            }
            indicator >>= 2;
        }
        profiles
    }

    pub fn create_label_string(indicator: usize) -> String {
        Self::label_string(&Self::from_bits(indicator))
    }

    pub const fn value(self) -> usize {
        match self {
            VerticalProfile::Empty => 1,
            VerticalProfile::Stackable => 2,
            VerticalProfile::Full => 3,
        }
    }

    pub fn parse_from(pattern: String) -> Result<Vec<Self>, VerticalProfileParseError> {
        let mut sequence = Vec::new();
        for char in pattern.chars() {
            sequence.push(match char {
                's' => VerticalProfile::Stackable,
                'e' => VerticalProfile::Empty,
                'f' => VerticalProfile::Full,
                _ => return Err(VerticalProfileParseError::InvalidVerticalPattern),
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
