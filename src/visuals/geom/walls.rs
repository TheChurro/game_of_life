
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
    pub fn compatible_below(self) -> usize {
        match self {
            WallProfile::Empty => WallProfile::Empty.to_bits() | WallProfile::Top.to_bits(),
            WallProfile::Ramp => WallProfile::Empty.to_bits(),
            WallProfile::Pmar => WallProfile::Empty.to_bits(),
            WallProfile::Wall => WallProfile::Empty.to_bits(),
            WallProfile::Llaw => WallProfile::Empty.to_bits(),
            WallProfile::Bottom => WallProfile::Empty.to_bits() | WallProfile::Top.to_bits(),
            WallProfile::Top => WallProfile::Empty.to_bits(),
        }
    }

    pub fn compatible_above(self) -> usize {
        match self {
            WallProfile::Empty => WallProfile::Empty.to_bits() | WallProfile::Bottom.to_bits(),
            WallProfile::Ramp => WallProfile::Empty.to_bits(),
            WallProfile::Pmar => WallProfile::Empty.to_bits(),
            WallProfile::Wall => WallProfile::Empty.to_bits(),
            WallProfile::Llaw => WallProfile::Empty.to_bits(),
            WallProfile::Bottom => WallProfile::Empty.to_bits(),
            WallProfile::Top => WallProfile::Empty.to_bits() | WallProfile::Bottom.to_bits(),
        }
    }

    pub fn compatible_across(self) -> usize {
        match self {
            WallProfile::Empty => WallProfile::Empty.to_bits() | WallProfile::Bottom.to_bits() | WallProfile::Top.to_bits(),
            WallProfile::Ramp => WallProfile::Pmar.to_bits(),
            WallProfile::Pmar => WallProfile::Ramp.to_bits(),
            WallProfile::Wall => WallProfile::Llaw.to_bits(),
            WallProfile::Llaw => WallProfile::Wall.to_bits(),
            WallProfile::Bottom => WallProfile::Empty.to_bits() | WallProfile::Bottom.to_bits(),
            WallProfile::Top => WallProfile::Empty.to_bits() | WallProfile::Top.to_bits(),
        }
    }

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

    #[allow(unused)]
    pub const fn to_bits(self) -> usize {
        match self {
            WallProfile::Empty =>  0x01,
            WallProfile::Ramp =>   0x02,
            WallProfile::Pmar =>   0x04,
            WallProfile::Wall =>   0x08,
            WallProfile::Llaw =>   0x10,
            WallProfile::Bottom => 0x11,
            WallProfile::Top =>    0x12,
        }
    }

    #[allow(unused)]
    pub fn from_bits(value: usize) -> Vec<WallProfile> {
        [WallProfile::Empty, WallProfile::Ramp, WallProfile::Pmar, WallProfile::Wall, WallProfile::Wall, WallProfile::Bottom, WallProfile::Top].iter().filter(|profile| profile.to_bits() & value != 0).cloned().collect()
    }
}