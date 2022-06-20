use std::f32::consts::FRAC_PI_3;

use bevy::math::{IVec2, Quat, Vec2, Vec3Swizzles};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TilingKind {
    Square,
    Hexagonal,
    OctagonAndSquare,
    EquilateralTriangular,
    RightTriangular,
}

#[derive(Debug, Clone)]
pub struct Tiling {
    pub kind: TilingKind,
    pub max_index: IVec2,
    pub offset: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EquilateralDirection {
    Up,
    Down,
}

impl EquilateralDirection {
    pub fn angle(self) -> f32 {
        match self {
            EquilateralDirection::Up => -std::f32::consts::FRAC_PI_6,
            EquilateralDirection::Down => std::f32::consts::FRAC_PI_6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RightTriangleRotation {
    Zero,
    One,
    Two,
    Three,
}

impl RightTriangleRotation {
    pub fn rotate(self, vec: [f32; 3]) -> [f32; 3] {
        match self {
            RightTriangleRotation::Zero => vec,
            RightTriangleRotation::One => [vec[1], -vec[0], vec[2]],
            RightTriangleRotation::Two => [-vec[0], -vec[1], vec[2]],
            RightTriangleRotation::Three => [-vec[1], vec[0], vec[2]],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileShape {
    Square,
    Hexagon,
    Octagon,
    EquilateralTriangle(EquilateralDirection),
    RightTriangle(RightTriangleRotation),
}

impl TileShape {
    #[inline]
    pub fn get_height(&self) -> f32 {
        match self {
            TileShape::Square => 1.0,
            TileShape::Hexagon => 2.0,
            TileShape::Octagon => 1.0 + std::f32::consts::SQRT_2,
            TileShape::EquilateralTriangle(_) => 1.5,
            TileShape::RightTriangle(_) => OCTAGON_SQUARE_DIFFERENCE_OF_CENTER,
        }
    }

    #[inline]
    pub fn get_width(&self) -> f32 {
        match self {
            TileShape::Square => 1.0,
            TileShape::Hexagon => 3.0f32.sqrt(),
            TileShape::Octagon => 1.0 + std::f32::consts::SQRT_2,
            TileShape::EquilateralTriangle(_) => TileShape::Hexagon.get_width(),
            TileShape::RightTriangle(_) => OCTAGON_SQUARE_DIFFERENCE_OF_CENTER,
        }
    }

    pub fn get_radius(&self) -> f32 {
        match self {
            TileShape::Square => std::f32::consts::SQRT_2 / 2.0,
            TileShape::Hexagon => 1.0,
            TileShape::Octagon => (1.0 + std::f32::consts::FRAC_1_SQRT_2).sqrt(),
            TileShape::EquilateralTriangle(_) => 1.0,
            // This is a fib.. but there isn't a great value for "radius" since this is not a n
            // equilateral shape unlike the other tile shapes...
            TileShape::RightTriangle(_) => 1.0,
        }
    }

    pub const fn get_side_count(&self) -> u32 {
        match self {
            TileShape::Square => 4,
            TileShape::Hexagon => 6,
            TileShape::Octagon => 8,
            TileShape::EquilateralTriangle(_) => 3,
            TileShape::RightTriangle(_) => 3,
        }
    }

    pub fn get_name(&self) -> String {
        match self {
            TileShape::Square => "Square".into(),
            TileShape::Hexagon => "Hexagon".into(),
            TileShape::Octagon => "Octagon".into(),
            TileShape::EquilateralTriangle(_) => "Equilateral Triangle".into(),
            TileShape::RightTriangle(_) => "Right Triangle".into(),
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
            TilingKind::Hexagonal => Vec2::new(
                TileShape::Hexagon.get_width() * self.max_index.x as f32,
                TileShape::Hexagon.get_height() + (self.max_index.y - 1) as f32 * 1.5,
            ),
            TilingKind::OctagonAndSquare => {
                self.max_index.as_vec2() * OCTAGON_SQUARE_DIFFERENCE_OF_CENTER
            }
            TilingKind::EquilateralTriangular => Vec2::new(
                TileShape::EquilateralTriangle(EquilateralDirection::Up).get_width()
                    * (0.5 + 0.5 * self.max_index.x as f32),
                TileShape::EquilateralTriangle(EquilateralDirection::Up).get_height()
                    * self.max_index.y as f32,
            ),
            TilingKind::RightTriangular => Vec2::new(
                TileShape::RightTriangle(RightTriangleRotation::Zero).get_width()
                    * ((self.max_index.x + 1) / 2) as f32,
                TileShape::RightTriangle(RightTriangleRotation::Zero).get_height()
                    * self.max_index.y as f32,
            ),
        }
    }

    pub fn adjust_position(&self, position: Vec2) -> Vec2 {
        let size = self.size();
        Vec2::new(
            (position.x - self.offset.x).rem_euclid(size.x) + self.offset.x,
            (position.y - self.offset.y).rem_euclid(size.y) + self.offset.y,
        )
    }

    pub fn in_bounds(&self, index: IVec2) -> bool {
        index.x >= 0 && index.x < self.max_index.x && index.y >= 0 && index.y < self.max_index.y
    }

    pub fn get_verticies(&self, index: IVec2, self_is_dual: bool) -> Vec<IVec2> {
        match self.kind {
            TilingKind::Square => {
                if self_is_dual {
                    vec![
                        index + IVec2::new(-1, 0),
                        index,
                        index + IVec2::new(0, -1),
                        index + IVec2::new(-1, -1),
                    ]
                } else {
                    vec![
                        index,
                        index + IVec2::new(1, 0),
                        index + IVec2::new(1, 1),
                        index + IVec2::new(0, 1),
                    ]
                }
            }
            TilingKind::Hexagonal => panic!("Not yet implemented"),
            TilingKind::OctagonAndSquare => panic!("Not yet implemented"),
            TilingKind::EquilateralTriangular => panic!("Not yet implemented"),
            TilingKind::RightTriangular => panic!("Not yet implemented"),
        }
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
        self.compute_offset_between_indicies(IVec2::ZERO, self.adjust_index(index)) + self.offset
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
            TilingKind::EquilateralTriangular => {
                // We do have to div_euclid here so 2 * double_x_change + index0.x <= index1.x
                // and the same for y.
                let double_x_change = index_offset.x.div_euclid(2);
                let single_x_change = index_offset.x.rem_euclid(2);
                let y_added_from_x = (index0.x + index0.y).rem_euclid(2) as f32 - 0.5;
                let double_y_change = index_offset.y.div_euclid(2);
                let single_y_change = index_offset.y.rem_euclid(2);
                let y_added_step =
                    ((index0.x + index0.y + single_x_change).rem_euclid(2)) as f32 + 1.0;
                Vec2::new(
                    TileShape::Hexagon.get_width()
                        * (double_x_change as f32 + single_x_change as f32 * 0.5),
                    TileShape::EquilateralTriangle(EquilateralDirection::Up).get_height()
                        * 2.0
                        * double_y_change as f32
                        + y_added_from_x * single_x_change as f32
                        + y_added_step * single_y_change as f32,
                )
            }
            // We always place the "center" of the tile in center of the rectangle split between
            // two right triangles making up a square. We control the rotation of said triangles
            // around that center in the triangle shape.
            TilingKind::RightTriangular => {
                Vec2::new(
                    (index1.x.div_euclid(2) - index0.x.div_euclid(2)) as f32,
                    (index1.y - index0.y) as f32,
                ) * OCTAGON_SQUARE_DIFFERENCE_OF_CENTER
            }
        }
    }

    pub fn get_index_for_position(&self, position: Vec2) -> IVec2 {
        let adjusted_position = position - self.offset;
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
            TilingKind::EquilateralTriangular => {
                let right_rotation = Vec2::new((-2.0 * FRAC_PI_3).cos(), (-2.0 * FRAC_PI_3).sin());
                let rotated_right = adjusted_position * right_rotation.x
                    + adjusted_position.perp() * right_rotation.y;
                let left_right = adjusted_position * right_rotation.x
                    - adjusted_position.perp() * right_rotation.y;

                let down_right = (1.0 + rotated_right.y).div_euclid(1.5) as i32;
                let down_left = (1.0 + left_right.y).div_euclid(1.5) as i32;
                let up_index = (1.0 + adjusted_position.y).div_euclid(1.5) as i32;
                IVec2::new(down_left - down_right, up_index)
            }
            TilingKind::RightTriangular => {
                let adjusted_position = adjusted_position
                    + 0.5
                        * Vec2::new(
                            OCTAGON_SQUARE_DIFFERENCE_OF_CENTER,
                            OCTAGON_SQUARE_DIFFERENCE_OF_CENTER,
                        );
                let x_block = adjusted_position
                    .x
                    .div_euclid(OCTAGON_SQUARE_DIFFERENCE_OF_CENTER)
                    as i32;
                let x_in_block = adjusted_position
                    .x
                    .rem_euclid(OCTAGON_SQUARE_DIFFERENCE_OF_CENTER);
                let y_block = adjusted_position
                    .y
                    .div_euclid(OCTAGON_SQUARE_DIFFERENCE_OF_CENTER)
                    as i32;
                let y_in_block = adjusted_position
                    .y
                    .rem_euclid(OCTAGON_SQUARE_DIFFERENCE_OF_CENTER);
                let triangle_in_block = if (x_block + y_block).rem_euclid(2) == 0 {
                    if x_in_block <= OCTAGON_SQUARE_DIFFERENCE_OF_CENTER - y_in_block {
                        0
                    } else {
                        1
                    }
                } else {
                    if x_in_block <= y_in_block {
                        0
                    } else {
                        1
                    }
                };

                IVec2::new(2 * x_block + triangle_in_block, y_block)
            }
        })
    }

    pub fn get_tile_containing(&self, position: Vec2) -> Tile {
        self.get_tile_at_index(self.get_index_for_position(position))
    }

    pub fn get_tile_at_index(&self, index: IVec2) -> Tile {
        Tile {
            position: self.get_position_from_index(index),
            index,
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
                TilingKind::EquilateralTriangular => {
                    TileShape::EquilateralTriangle(if (index.x + index.y) % 2 == 0 {
                        EquilateralDirection::Down
                    } else {
                        EquilateralDirection::Up
                    })
                }
                TilingKind::RightTriangular => TileShape::RightTriangle(
                    match (
                        (index.x.div_euclid(2) + index.y) % 2 == 0,
                        index.x.rem_euclid(2) == 0,
                    ) {
                        (true, true) => RightTriangleRotation::Zero,
                        (true, false) => RightTriangleRotation::Two,
                        (false, true) => RightTriangleRotation::One,
                        (false, false) => RightTriangleRotation::Three,
                    },
                ),
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
            TilingKind::EquilateralTriangular => {
                if (index.x + index.y) % 2 == 0 {
                    &[
                        (-2, 1),
                        (-1, 1),
                        (0, 1),
                        (1, 1),
                        (2, 1),
                        (-2, 0),
                        (-1, 0),
                        (1, 0),
                        (2, 0),
                        (-1, -1),
                        (0, -1),
                        (1, -1),
                    ]
                } else {
                    &[
                        (-1, 1),
                        (0, 1),
                        (1, 1),
                        (-2, 0),
                        (-1, 0),
                        (1, 0),
                        (2, 0),
                        (-2, -1),
                        (-1, -1),
                        (0, -1),
                        (1, -1),
                        (2, -1),
                    ]
                }
            }
            TilingKind::RightTriangular => match (
                (index.x.div_euclid(2) + index.y) % 2 == 0,
                index.x.rem_euclid(2) == 0,
            ) {
                (true, true) => &[
                    (-2, 1),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                    (-2, 0),
                    (-1, 0),
                    (1, 0),
                    (2, 0),
                    (3, 0),
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (2, -1),
                    (3, -1),
                ],
                (true, false) => &[
                    (-3, 1),
                    (-2, 1),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                    (-3, 0),
                    (-2, 0),
                    (-1, 0),
                    (1, 0),
                    (2, 0),
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (2, -1),
                ],
                (false, true) => &[
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                    (2, 1),
                    (3, 1),
                    (-2, 0),
                    (-1, 0),
                    (1, 0),
                    (2, 0),
                    (3, 0),
                    (-2, -1),
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                ],
                (false, false) => &[
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                    (2, 1),
                    (-3, 0),
                    (-2, 0),
                    (-1, 0),
                    (1, 0),
                    (2, 0),
                    (-3, -1),
                    (-2, -1),
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                ],
            },
        }
    }

    pub fn get_dual(&self) -> Self {
        match self.kind {
            TilingKind::Square => Self {
                kind: TilingKind::Square,
                offset: Vec2::new(-0.5, -0.5),
                max_index: self.max_index + IVec2::new(1, 1),
            },
            TilingKind::Hexagonal => Self {
                kind: TilingKind::EquilateralTriangular,
                offset: Vec2::new(
                    -TileShape::EquilateralTriangle(EquilateralDirection::Up).get_width() * 0.5,
                    -0.5,
                ),
                max_index: self.max_index * IVec2::new(2, 1) + IVec2::new(1, 1),
            },
            TilingKind::OctagonAndSquare => Self {
                kind: TilingKind::RightTriangular,
                offset: Vec2::new(
                    -0.5 * OCTAGON_SQUARE_DIFFERENCE_OF_CENTER,
                    -0.5 * OCTAGON_SQUARE_DIFFERENCE_OF_CENTER,
                ),
                max_index: self.max_index * IVec2::new(2, 1) + IVec2::new(4, 2),
            },
            TilingKind::EquilateralTriangular => Self {
                kind: TilingKind::Hexagonal,
                offset: Vec2::new(
                    -TileShape::EquilateralTriangle(EquilateralDirection::Up).get_width(),
                    -1.0,
                ),
                max_index: self.max_index + IVec2::new(2, 2),
            },
            TilingKind::RightTriangular => Self {
                kind: TilingKind::OctagonAndSquare,
                offset: Vec2::new(
                    -0.5 * OCTAGON_SQUARE_DIFFERENCE_OF_CENTER,
                    -0.5 * OCTAGON_SQUARE_DIFFERENCE_OF_CENTER,
                ),
                max_index: IVec2::new(self.max_index.x / 2 + 1, self.max_index.y + 1),
            },
        }
    }

    pub fn get_adjacent(&self, _index: IVec2) -> &'static [(i32, i32, usize)] {
        match self.kind {
            TilingKind::Square => &[(1, 0, 2), (0, 1, 3), (-1, 0, 0), (0, -1, 1), ],
            TilingKind::Hexagonal => todo!(),// &[(0, 1, 3), (1, 1, 4), (-1, 0, 5), (1, 0, 0), (-1, -1, 1), (0, -1, 2)],
            TilingKind::OctagonAndSquare => {
                todo!()
                // if (index.x + index.y) % 2 == 0 {
                //     &[(-1, 0), (0, -1), (1, 0), (0, 1)]
                // } else {
                //     &[
                //         (-1, -1),
                //         (-1, 0),
                //         (-1, 1),
                //         (0, 1),
                //         (1, 1),
                //         (1, 0),
                //         (1, -1),
                //         (0, -1),
                //     ]
                // }
            }
            TilingKind::EquilateralTriangular => {
                todo!()
                // if (index.x + index.y) % 2 == 0 {
                //     &[(-1, 0), (1, 0), (0, 1)]
                // } else {
                //     &[(-1, 0), (1, 0), (0, -1)]
                // }
            }
            TilingKind::RightTriangular => todo!()
            // match (
            //     (index.x.div_euclid(2) + index.y) % 2 == 0,
            //     index.x.rem_euclid(2) == 0,
            // ) {
            //     (true, true) => &[(-1, 0), (1, 0), (0, -1)],
            //     (true, false) => &[(-1, 0), (1, 0), (0, 1)],
            //     (false, true) => &[(-1, 0), (1, 0), (0, 1)],
            //     (false, false) => &[(-1, 0), (1, 0), (0, -1)],
            // },
        }
    }
}
