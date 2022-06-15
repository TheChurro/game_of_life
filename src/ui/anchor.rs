use bevy::{
    math::Vec3,
    prelude::{Added, Component, EventReader, Query, Res, Transform},
    window::{WindowResized, Windows},
};

use super::element::UiElement;

/// A component used for laying out elements within the screen.
#[derive(Component)]
pub struct AnchoredUi {
    /// How far to the left will this element be placed.
    /// 0 means the left edge of this component touches
    /// the left edge of the screen and 1 means the same
    /// for the right edges.
    pub x_percent: f32,
    /// How far to the down will this element be placed.
    /// 0 means the bottom edge of this component touches
    /// the bottom edge of the screen and 1 means the same
    /// for the top edges.
    pub y_percent: f32,
    /// If given a value, this will cause the element to fill
    /// the set ratio of the screen's width.
    pub width_grow: Option<f32>,
    /// If given a value, this will cause the element to fill
    /// the set ratio of the screen's height.
    pub height_grow: Option<f32>,
}

/// When adding an element with an anchor, adjust it's transform to be positioned
/// correctly within the window.
pub fn position_on_added(
    windows: Res<Windows>,
    mut transform_query: Query<(&mut Transform, &mut UiElement, &AnchoredUi), Added<AnchoredUi>>,
) {
    if let Some(window) = windows.get_primary() {
        transform_query.for_each_mut(|(mut transform, mut element, anchor)| {
            if let Some(percent) = anchor.width_grow {
                element.size.width = percent * window.width();
            }
            if let Some(percent) = anchor.height_grow {
                element.size.height = percent * window.height();
            }

            transform.translation = Vec3::new(
                (anchor.x_percent - 0.5) * (window.width() - element.size.width),
                (anchor.y_percent - 0.5) * (window.height() - element.size.height),
                transform.translation.z,
            );
        });
    }
}

/// When the window's size changes, adjust the transform of anchored ui elements
/// so they are correctly positioned within the window.
pub fn position_on_window_changed(
    mut window_resize: EventReader<WindowResized>,
    mut transform_query: Query<(&mut Transform, &mut UiElement, &AnchoredUi)>,
) {
    for resize in window_resize.iter() {
        transform_query.for_each_mut(|(mut transform, mut element, anchor)| {
            if let Some(percent) = anchor.width_grow {
                element.size.width = percent * resize.width;
            }
            if let Some(percent) = anchor.height_grow {
                element.size.height = percent * resize.height;
            }

            transform.translation = Vec3::new(
                (anchor.x_percent - 0.5) * (resize.width - element.size.width),
                (anchor.y_percent - 0.5) * (resize.height - element.size.height),
                transform.translation.z,
            );
        });
    }
}
