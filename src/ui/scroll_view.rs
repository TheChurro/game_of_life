use bevy::{
    hierarchy::Children,
    math::{Size, Vec2, Vec3},
    prelude::{Changed, Component, Entity, Query, Transform},
};

use super::element::UiElement;

/// What direction to layout the children of Scrollers
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

/// A component for UI Elements that allows scrolling their children
/// along the length of this element.
#[derive(Component)]
pub struct UiLinearScroll {
    pub scroll_position: Vec2,
    pub layout_direction: LayoutDirection,
}

impl Default for UiLinearScroll {
    fn default() -> Self {
        Self {
            scroll_position: Vec2::ZERO,
            layout_direction: LayoutDirection::Vertical,
        }
    }
}

/// Helper function which takes in a list of children,
/// calculates the bounds needed to fit them along the
/// layout direction of the passed in scroll, and positions
/// them accordingly.
fn position_scroll_children(
    children: &Children,
    transform_query: &mut Query<(&mut Transform, &UiElement)>,
    bounding_size: Size,
    scroll: &mut UiLinearScroll,
) {
    let mut width = 0.0;
    let mut height = 0.0;

    for child in children.iter() {
        if let Ok((_, element)) = transform_query.get(*child) {
            match scroll.layout_direction {
                LayoutDirection::Vertical => {
                    width = element.size.width.max(width);
                    height += element.size.height;
                }
                LayoutDirection::Horizontal => {
                    width += element.size.width;
                    height = element.size.height.max(height);
                }
            }
        }
    }

    // Update scroll so that we cannot scroll past the bounds of our children.
    scroll.scroll_position.y = scroll
        .scroll_position
        .y
        .min((height - bounding_size.height).max(0.0))
        .max(0.0);
    scroll.scroll_position.x = scroll
        .scroll_position
        .x
        .min((width - bounding_size.width).max(0.0))
        .max(0.0);

    let mut position = Vec3::new(
        -bounding_size.width / 2.0 - scroll.scroll_position.x,
        bounding_size.height / 2.0 + scroll.scroll_position.y,
        1.0,
    );

    for child in children.iter() {
        if let Ok((mut transform, element)) = transform_query.get_mut(*child) {
            transform.translation =
                position + Vec3::new(element.size.width / 2.0, -element.size.height / 2.0, 0.0);
            position += match scroll.layout_direction {
                LayoutDirection::Vertical => Vec3::new(0.0, -element.size.height, 0.0),
                LayoutDirection::Horizontal => Vec3::new(element.size.width, 0.0, 0.0),
            }
        }
    }
}

/// Position the children of this linear scroll when the children change.
pub fn linear_scroll_children_changed(
    mut transform_query: Query<(&mut Transform, &UiElement)>,
    mut scroll_query: Query<(Entity, &mut UiLinearScroll, &Children), Changed<Children>>,
) {
    scroll_query.for_each_mut(|(entity, mut scroll, children)| {
        let size = match transform_query.get(entity) {
            Ok((_, element)) => element.size.clone(),
            Err(_) => Size::new(0.0, 0.0),
        };

        position_scroll_children(children, &mut transform_query, size, &mut scroll);
    });
}

/// Position the children of this linear scroll when this element changes.
pub fn linear_scroll_handler(
    mut transform_query: Query<(&mut Transform, &UiElement)>,
    mut scroll_query: Query<
        (Entity, &mut UiLinearScroll, &UiElement, &Children),
        Changed<UiElement>,
    >,
) {
    scroll_query.for_each_mut(|(entity, mut scroll, element, children)| {
        scroll.scroll_position += element.scroll_state.current * Vec2::new(1.0, -1.0);

        let size = match transform_query.get(entity) {
            Ok((_, element)) => element.size.clone(),
            Err(_) => Size::new(0.0, 0.0),
        };

        position_scroll_children(children, &mut transform_query, size, &mut scroll);
    });
}
