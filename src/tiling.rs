use bevy::math::{IVec2, Quat, Vec2, Vec3Swizzles};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TilingKind {
    Square,
    Hexagonal,
    OctagonAndSquare,
}

#[derive(Debug)]
pub struct Tiling {
    pub kind: TilingKind,
    pub max_index: IVec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileShape {
    Square,
    Hexagon,
    Octagon,
}

impl TileShape {
    #[inline]
    pub fn get_height(&self) -> f32 {
        match self {
            TileShape::Square => 1.0,
            TileShape::Hexagon => 2.0,
            TileShape::Octagon => 1.0 + std::f32::consts::SQRT_2,
        }
    }

    #[inline]
    pub fn get_width(&self) -> f32 {
        match self {
            TileShape::Square => 1.0,
            TileShape::Hexagon => 3.0f32.sqrt(),
            TileShape::Octagon => 1.0 + std::f32::consts::SQRT_2,
        }
    }

    pub fn get_radius(&self) -> f32 {
        match self {
            TileShape::Square => std::f32::consts::SQRT_2 / 2.0,
            TileShape::Hexagon => 1.0,
            TileShape::Octagon => (1.0 + std::f32::consts::FRAC_1_SQRT_2).sqrt(),
        }
    }

    pub fn get_side_count(&self) -> u32 {
        match self {
            TileShape::Square => 4,
            TileShape::Hexagon => 6,
            TileShape::Octagon => 8,
        }
    }
}

pub struct Tile {
    pub position: Vec2,
    pub shape: TileShape,
    pub index: IVec2,
}

pub const HEXAGON_AXIS_RIGHT: f32 = std::f32::consts::FRAC_PI_3;
pub const HEXAGON_AXIS_LEFT: f32 = -std::f32::consts::FRAC_PI_3;
pub const OCTAGON_SQUARE_DIFFERENCE_OF_CENTER: f32 = (2.0 + std::f32::consts::SQRT_2) / 2.0;

// Determine which band of hexagons going along the given axis this position lies in.
// If the position is not in the "core" of the hexagon (within the bounds of the side
// of a hexagon that the axis points to) then we return false.
pub fn get_hexagon_band_perpendicular_to(position: Vec2, axis: Vec2) -> (i32, i32, bool) {
    let distance_perpendicular_to = axis.perp_dot(position);
    let band = (distance_perpendicular_to + 0.5).div_euclid(1.5);
    let position_in_band = (distance_perpendicular_to + 0.5).rem_euclid(1.5);
    let segment_in_band = ((position.dot(axis)
        + (band as f32 + 1.0) * TileShape::Hexagon.get_width() / 2.0)
        / TileShape::Hexagon.get_width())
    .floor() as i32;
    (band as i32, segment_in_band, position_in_band < 1.0)
}

impl Tiling {
    pub fn size(&self) -> Vec2 {
        match self.kind {
            TilingKind::Square => self.max_index.as_vec2(),
            TilingKind::Hexagonal => {
                Vec2::new(
                    TileShape::Hexagon.get_width() * self.max_index.x as f32,
                    TileShape::Hexagon.get_height() + (self.max_index.y - 1) as f32 * 1.5,
                )
            }
            TilingKind::OctagonAndSquare => {
                self.max_index.as_vec2() * OCTAGON_SQUARE_DIFFERENCE_OF_CENTER
            }
        }
    }

    pub fn adjust_position(&self, position: Vec2) -> Vec2 {
        let size = self.size();
        Vec2::new(position.x.rem_euclid(size.x), position.y.rem_euclid(size.y))
    }

    pub fn adjust_index(&self, index: IVec2) -> IVec2 {
        match self.kind {
            TilingKind::Hexagonal => {
                //Hexagonal tilings are annoying because moving upwards also moves you sideways, but the way in
                // which one moves sideways is non-obvious. Every 2 moved upwards moves you one to the left.
                let target_y = index.y.rem_euclid(self.max_index.y);
                let over_count = (target_y - index.y) / 2;
                IVec2::new(
                    (index.x + over_count).rem_euclid(self.max_index.x),
                    target_y,
                )
            }
            _ => IVec2::new(
                index.x.rem_euclid(self.max_index.x),
                index.y.rem_euclid(self.max_index.y),
            ),
        }
    }

    pub fn get_position_from_index(&self, index: IVec2) -> Vec2 {
        self.compute_offset_between_indicies(IVec2::ZERO, self.adjust_index(index))
    }

    pub fn compute_offset_between_indicies(&self, index0: IVec2, index1: IVec2) -> Vec2 {
        let index_offset = index1 - index0;
        match self.kind {
            TilingKind::Square => index_offset.as_vec2(),
            TilingKind::Hexagonal => {
                Vec2::new(TileShape::Hexagon.get_width() * index_offset.x as f32, 0.0)
                    + index_offset.y as f32
                        * Vec2::new(
                            -0.5 * TileShape::Hexagon.get_width(),
                            (TileShape::Hexagon.get_height() + 1.0) / 2.0,
                        )
            }
            TilingKind::OctagonAndSquare => {
                index_offset.as_vec2() * OCTAGON_SQUARE_DIFFERENCE_OF_CENTER
            }
        }
    }

    pub fn get_index_for_position(&self, position: Vec2) -> IVec2 {
        let adjusted_position = self.adjust_position(position);
        self.adjust_index(match self.kind {
            TilingKind::Square => adjusted_position.round().as_ivec2(),
            TilingKind::Hexagonal => {
                if let (vertical_band, segment_in_band, true) =
                    get_hexagon_band_perpendicular_to(adjusted_position, Vec2::new(1.0, 0.0))
                {
                    IVec2::new(segment_in_band, vertical_band)
                } else if let (up_right_band, up_right_segment, true) =
                    get_hexagon_band_perpendicular_to(
                        adjusted_position,
                        Vec2::new(HEXAGON_AXIS_RIGHT.cos(), HEXAGON_AXIS_RIGHT.sin()),
                    )
                {
                    IVec2::new(up_right_segment - up_right_band, up_right_segment)
                } else {
                    let (up_left_band, up_left_segment, _) = get_hexagon_band_perpendicular_to(
                        adjusted_position,
                        Vec2::new(HEXAGON_AXIS_LEFT.cos(), HEXAGON_AXIS_LEFT.sin()),
                    );
                    IVec2::new(up_left_band, up_left_band - up_left_segment)
                }
            }
            TilingKind::OctagonAndSquare => {
                // First determine if this point is in the bounds of a square of this tiling.
                let small_indicies = ((adjusted_position + Vec2::new(0.5, 0.5))
                    / OCTAGON_SQUARE_DIFFERENCE_OF_CENTER)
                    .floor()
                    .as_ivec2();
                let small_pos = small_indicies.as_vec2() * OCTAGON_SQUARE_DIFFERENCE_OF_CENTER;
                let pos = adjusted_position + Vec2::new(0.5, 0.5) - small_pos;
                if (small_indicies.x + small_indicies.y) % 2 == 0 && pos.x < 1.0 && pos.y < 1.0 {
                    small_indicies
                } else {
                    // Now, we are going to treat the octagons as a square tiling of the grid at a 45 degree angle
                    // and then adjust our positions from there...
                    let rotated_position = (Quat::from_rotation_z(-std::f32::consts::FRAC_PI_4)
                        * adjusted_position.extend(0.0))
                    .xy();
                    let rotated_index = (rotated_position / TileShape::Octagon.get_height())
                        .floor()
                        .as_ivec2();
                    IVec2::new(
                        0 + rotated_index.x - rotated_index.y,
                        1 + rotated_index.x + rotated_index.y,
                    )
                }
            }
        })
    }

    pub fn get_tile_containing(&self, position: Vec2) -> Tile {
        self.get_tile_at_index(self.get_index_for_position(position))
    }

    pub fn get_tile_at_index(&self, index: IVec2) -> Tile {
        Tile {
            position: self.get_position_from_index(index),
            index: index,
            shape: match self.kind {
                TilingKind::Square => TileShape::Square,
                TilingKind::Hexagonal => TileShape::Hexagon,
                TilingKind::OctagonAndSquare => {
                    if (index.x + index.y) % 2 == 0 {
                        TileShape::Square
                    } else {
                        TileShape::Octagon
                    }
                }
            },
        }
    }

    pub fn get_neighbors(&self, index: IVec2) -> &'static [(i32, i32)] {
        match self.kind {
            TilingKind::Square => &[
                (-1, -1),
                (-1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
                (1, 0),
                (1, -1),
                (0, -1),
            ],
            TilingKind::Hexagonal => &[(0, 1), (1, 1), (-1, 0), (1, 0), (-1, -1), (0, -1)],
            TilingKind::OctagonAndSquare => {
                if (index.x + index.y) % 2 == 0 {
                    &[(-1, 0), (0, -1), (1, 0), (0, 1)]
                } else {
                    &[
                        (-1, -1),
                        (-1, 0),
                        (-1, 1),
                        (0, 1),
                        (1, 1),
                        (1, 0),
                        (1, -1),
                        (0, -1),
                    ]
                }
            }
        }
    }
}
