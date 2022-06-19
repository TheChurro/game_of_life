
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