use bevy::{
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    math::Size,
    prelude::{Color, Commands, Component, Entity, EventReader, Mut, Query, Res, With},
    transform::TransformBundle,
};

use crate::{
    ui::{LayoutDirection, UiElement, UiLinearScroll},
    visuals::{
        collapse::CollapseEntry,
        geom::{handles::GeometryHandleSet, GeomOrientation, GeometryStorage, WallProfile},
    },
};

use super::{MenuState, HEADER_FONT_SIZE, HEADER_HEIGHT, REGULAR_FONT_SIZE, REGULAR_HEIGHT_STEP};

#[derive(Component, Clone, Debug)]
pub struct Inspect(pub Entity);

#[derive(Component)]
pub struct TileInspector {}

pub fn inspect(
    mut events: EventReader<Inspect>,
    menu_data: Res<MenuState>,
    geom_data: Res<GeometryStorage>,
    mut collapse_query: Query<&mut CollapseEntry>,
    inspector_query: Query<Entity, With<TileInspector>>,
    mut commands: Commands,
) {
    for event in events.iter() {
        let mut entry: Mut<CollapseEntry> = match collapse_query.get_mut(event.0) {
            Ok(entry) => entry,
            Err(_) => return,
        };

        entry.history_enabled = true;

        let inspector = inspector_query.single();
        let mut inspector = commands.entity(inspector);
        inspector.despawn_descendants();
        inspector.with_children(|children_spawner| {
            let width = 300.0;

            // Add text letting us know which tile this is
            children_spawner
                .spawn_bundle(menu_data.get_text_bundle(
                    format!("Tile {}@{}", entry.index_in_tiling, entry.height),
                    HEADER_FONT_SIZE,
                    Color::BLACK,
                ))
                .insert(UiElement {
                    size: Size::new(width, HEADER_HEIGHT),
                    ..Default::default()
                });

            // Add text with selected mesh and it's corresponding sides
            if let Some(mesh) = entry.current_mesh {
                let mut mesh_root = children_spawner.spawn();
                menu_data.spawn_labeled(
                    &mut mesh_root,
                    Size::new(width / 2.0, REGULAR_HEIGHT_STEP),
                    "Mesh".to_string(),
                    Color::BLACK,
                    |menu_data, children| {
                        let text = match mesh.orientation {
                            GeomOrientation::Standard { rotations } => {
                                format!("{}@{}", mesh.index, rotations)
                            }
                            GeomOrientation::Flipped { rotations } => {
                                format!("{}@r{}", mesh.index, rotations)
                            }
                        };
                        children
                            .spawn_bundle(menu_data.get_text_bundle(
                                text,
                                REGULAR_FONT_SIZE,
                                Color::BLACK,
                            ))
                            .insert(UiElement {
                                size: Size::new(width / 2.0, REGULAR_HEIGHT_STEP),
                                ..Default::default()
                            });
                    },
                );

                let profile = &geom_data.profiles[mesh.index];
                for side in 0..profile.get_side_count() {
                    children_spawner
                        .spawn_bundle(menu_data.get_text_bundle(
                            profile.get_wall(side, mesh.orientation).label().to_string(),
                            REGULAR_FONT_SIZE,
                            Color::BLACK,
                        ))
                        .insert(UiElement {
                            size: Size::new(width, REGULAR_HEIGHT_STEP),
                            ..Default::default()
                        });
                }
            }

            // Add text for restrictions per side.
            for restriction in &entry.edge_restrictions {
                children_spawner
                    .spawn_bundle(menu_data.get_text_bundle(
                        format!("Edge {}", restriction.edge),
                        REGULAR_FONT_SIZE,
                        Color::BLACK,
                    ))
                    .insert(UiElement {
                        size: Size::new(width, REGULAR_HEIGHT_STEP),
                        ..Default::default()
                    });
                for (restriction_bits, label) in [
                    (
                        restriction.bottom_restriction.unwrap_or(0),
                        "Bottom".to_string(),
                    ),
                    (
                        restriction.level_restriction.unwrap_or(0),
                        "Level".to_string(),
                    ),
                    (restriction.top_restriction.unwrap_or(0), "Top".to_string()),
                ] {
                    let restrictions = WallProfile::from_bits(restriction_bits);
                    if restrictions.len() > 0 {
                        let mut restriction_root = children_spawner.spawn();
                        menu_data.spawn_labeled(
                            &mut restriction_root,
                            Size::new(width / 2.0, restrictions.len() as f32 * REGULAR_HEIGHT_STEP),
                            label,
                            Color::BLACK,
                            |menu_data, children| {
                                children
                                    .spawn_bundle(TransformBundle::default())
                                    .insert(UiElement {
                                        size: Size::new(
                                            width / 2.0,
                                            restrictions.len() as f32 * REGULAR_HEIGHT_STEP,
                                        ),
                                        ..Default::default()
                                    })
                                    .insert(UiLinearScroll {
                                        layout_direction: LayoutDirection::Vertical,
                                        ..Default::default()
                                    })
                                    .with_children(|children| {
                                        for restriction in restrictions {
                                            children
                                                .spawn_bundle(menu_data.get_text_bundle(
                                                    restriction.label().to_string(),
                                                    REGULAR_FONT_SIZE,
                                                    Color::BLACK,
                                                ))
                                                .insert(UiElement {
                                                    size: Size::new(
                                                        width / 2.0,
                                                        REGULAR_HEIGHT_STEP,
                                                    ),
                                                    ..Default::default()
                                                });
                                        }
                                    });
                            },
                        );
                    }
                }
            }

            children_spawner
                .spawn_bundle(menu_data.get_text_bundle(
                    "History".to_string(),
                    HEADER_FONT_SIZE,
                    Color::BLACK,
                ))
                .insert(UiElement {
                    size: Size::new(width, HEADER_HEIGHT),
                    ..Default::default()
                });
            for log in &entry.history {
                let text = format!("{}", log);
                children_spawner
                    .spawn_bundle(menu_data.get_text_bundle(text, REGULAR_FONT_SIZE, Color::BLACK))
                    .insert(UiElement {
                        size: Size::new(width, REGULAR_HEIGHT_STEP),
                        ..Default::default()
                    });
            }

            let set = entry.compute_edge_restrictions(&geom_data);
            let combined_restrictions = GeometryHandleSet::intersection(
                [&entry.possible_geometry_entries_from_corner_data]
                    .into_iter()
                    .chain(&set),
            );
            children_spawner
                .spawn_bundle(menu_data.get_text_bundle(
                    "Combined Restricted Handles".to_string(),
                    HEADER_FONT_SIZE,
                    Color::BLACK,
                ))
                .insert(UiElement {
                    size: Size::new(width, HEADER_HEIGHT),
                    ..Default::default()
                });
            for handle in combined_restrictions.into_iter() {
                let text = match handle.orientation {
                    GeomOrientation::Standard { rotations } => {
                        format!("{}@{}", handle.index, rotations)
                    }
                    GeomOrientation::Flipped { rotations } => {
                        format!("{}@r{}", handle.index, rotations)
                    }
                };
                children_spawner
                    .spawn_bundle(menu_data.get_text_bundle(text, REGULAR_FONT_SIZE, Color::BLACK))
                    .insert(UiElement {
                        size: Size::new(width, REGULAR_HEIGHT_STEP),
                        ..Default::default()
                    });
            }
            children_spawner
                .spawn_bundle(menu_data.get_text_bundle(
                    "Corners Restrictions".to_string(),
                    HEADER_FONT_SIZE,
                    Color::BLACK,
                ))
                .insert(UiElement {
                    size: Size::new(width, HEADER_HEIGHT),
                    ..Default::default()
                });
            for handle in &entry.possible_geometry_entries_from_corner_data {
                let text = match handle.orientation {
                    GeomOrientation::Standard { rotations } => {
                        format!("{}@{}", handle.index, rotations)
                    }
                    GeomOrientation::Flipped { rotations } => {
                        format!("{}@r{}", handle.index, rotations)
                    }
                };
                children_spawner
                    .spawn_bundle(menu_data.get_text_bundle(text, REGULAR_FONT_SIZE, Color::BLACK))
                    .insert(UiElement {
                        size: Size::new(width, REGULAR_HEIGHT_STEP),
                        ..Default::default()
                    });
            }

            for (i, restriction_set) in set.iter().enumerate() {
                children_spawner
                    .spawn_bundle(menu_data.get_text_bundle(
                        format!("Edge Restriction {}", i),
                        HEADER_FONT_SIZE,
                        Color::BLACK,
                    ))
                    .insert(UiElement {
                        size: Size::new(width, HEADER_HEIGHT),
                        ..Default::default()
                    });
                for handle in restriction_set {
                    let text = match handle.orientation {
                        GeomOrientation::Standard { rotations } => {
                            format!("{}@{}", handle.index, rotations)
                        }
                        GeomOrientation::Flipped { rotations } => {
                            format!("{}@r{}", handle.index, rotations)
                        }
                    };
                    children_spawner
                        .spawn_bundle(menu_data.get_text_bundle(
                            text,
                            REGULAR_FONT_SIZE,
                            Color::BLACK,
                        ))
                        .insert(UiElement {
                            size: Size::new(width, REGULAR_HEIGHT_STEP),
                            ..Default::default()
                        });
                }
            }
        });
    }
}
