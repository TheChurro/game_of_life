use bevy::{
    math::{Size, Vec2},
    prelude::{Changed, Component, Query},
    sprite::Sprite,
};

/// A component marking an entity as a UI Element.
/// This stores the size of the element as well
/// as states relevant to mouse interactions.
#[derive(Component)]
pub struct UiElement {
    /// Size of the element. Note: the ui element exists
    /// in the center of this size.
    pub size: Size,
    /// A state representing whether this element is hovered this frame
    pub hover_state: UiStateDetails<bool>,
    /// A state representing whether or not this element has is clicked.
    /// This is set to true when a left click starts over this element
    /// until the left click ends.
    pub click_state: UiStateDetails<bool>,
    /// A state representing whether or not this element has is selected.
    /// This is set at the start of a left click on this element and ends
    /// when a left click happens outside this element.
    pub selected_state: UiStateDetails<bool>,
    /// A state representing how much the mouse-wheel has scrolled while
    /// over this element.
    pub scroll_state: UiStateDetails<Vec2>,
}

impl Default for UiElement {
    fn default() -> Self {
        Self {
            size: Size::new(0.0, 0.0),
            hover_state: Default::default(),
            click_state: Default::default(),
            selected_state: Default::default(),
            scroll_state: Default::default(),
        }
    }
}

/// Represents some state in the ui that can change from
/// tick-to-tick as well as whether this state is enabled
/// for the ui element this lives in.
#[derive(Component)]
pub struct UiStateDetails<T> {
    pub current: T,
    pub previous: T,
    pub accepts_state: bool,
}

impl Default for UiStateDetails<bool> {
    fn default() -> Self {
        Self {
            current: false,
            previous: false,
            accepts_state: false,
        }
    }
}

impl Default for UiStateDetails<Vec2> {
    fn default() -> Self {
        Self {
            current: Vec2::ZERO,
            previous: Vec2::ZERO,
            accepts_state: false,
        }
    }
}

impl UiStateDetails<bool> {
    /// Did this state just stop having a "true" value
    pub fn exited(&self) -> bool {
        !self.current && self.previous
    }

    /// Did this state just start having a "true" value
    pub fn entered(&self) -> bool {
        self.current && !self.previous
    }
}

/// Update the size of sprites attached to ui elements when the ui element
/// changes its size.
pub fn update_sprite_to_match_layout(
    mut query: Query<(&mut Sprite, &UiElement), Changed<UiElement>>,
) {
    query.for_each_mut(|(mut sprite, element)| {
        sprite.custom_size = Some(Vec2::new(element.size.width, element.size.height));
    });
}
