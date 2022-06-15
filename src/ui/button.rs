use bevy::prelude::{Changed, Component, EventWriter, Handle, Image, Query};

use super::element::UiElement;

/// A component placed on a UI Element that emits the set
/// event when clicked (on click release)
#[derive(Component)]
pub struct Button<Event: Clone + Component> {
    pub default_image: Handle<Image>,
    pub hover_image: Option<Handle<Image>>,
    pub pressed_image: Option<Handle<Image>>,
    pub event: Event,
}

impl<Event: Clone + Component> Button<Event> {
    pub fn new(image: Handle<Image>, event: Event) -> Self {
        Self {
            default_image: image,
            hover_image: None,
            pressed_image: None,
            event,
        }
    }
}

/// Function that detects changes to the click state and updates the
/// visuals of the button and potentially sends the set event if
/// detecting the click ending.
pub fn button_handler<Event: Clone + Component>(
    mut query: Query<(&mut Handle<Image>, &Button<Event>, &UiElement), Changed<UiElement>>,
    mut events: EventWriter<Event>,
) {
    query.for_each_mut(|(mut image, button, element)| {
        if element.click_state.entered() {
            if let Some(pressed_image) = &button.pressed_image {
                *image = pressed_image.clone();
            }
        } else if element.click_state.exited() {
            events.send(button.event.clone());

            let mut updated_image = false;
            if element.hover_state.current {
                if let Some(hover_image) = &button.hover_image {
                    *image = hover_image.clone();
                    updated_image = true;
                }
            }
            if !updated_image {
                *image = button.default_image.clone();
            }
        } else if !element.click_state.current || button.pressed_image.is_none() {
            if let Some(hover_image) = &button.hover_image {
                if element.hover_state.entered() {
                    *image = hover_image.clone();
                } else if element.hover_state.exited() {
                    *image = button.default_image.clone();
                }
            }
        }
    })
}
