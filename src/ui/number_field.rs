use bevy::{
    input::Input,
    prelude::{Color, Component, EventWriter, KeyCode, Query, Res},
    text::{Text, TextSection, TextStyle},
};

use super::element::UiElement;

/// Trait for structs that can generate an event given a value.
pub trait NumberedEventGenerator {
    type Event: Component + Clone;
    fn create_event(&self, value: u32) -> Self::Event;
}

/// Component for UI Elements that allows for typing positive integers.
#[derive(Component)]
pub struct NumberField<EventGenerator: Component + NumberedEventGenerator> {
    /// A generator used to create the events when the value of the field changes.
    pub event_generator: EventGenerator,
    pub current_value: u32,
    pub max_value: u32,
    pub min_value: u32,
}

/// Detect button presses on selected number fields to type in numbers on them.
pub fn number_field_handler<EventGenerator: Component + NumberedEventGenerator>(
    mut query: Query<(&mut Text, &mut NumberField<EventGenerator>, &UiElement)>,
    mut events: EventWriter<EventGenerator::Event>,
    keyboard: Res<Input<KeyCode>>,
) {
    query.for_each_mut(|(mut text, mut number_field, element)| {
        if !element.selected_state.current {
            return;
        }
        let initial_value = number_field.current_value;
        if keyboard.just_released(KeyCode::Delete) || keyboard.just_released(KeyCode::Back) {
            number_field.current_value =
                (number_field.current_value / 10).max(number_field.min_value);
        }
        for (key, value) in &[
            (KeyCode::Key0, 0),
            (KeyCode::Key1, 1),
            (KeyCode::Key2, 2),
            (KeyCode::Key3, 3),
            (KeyCode::Key4, 4),
            (KeyCode::Key5, 5),
            (KeyCode::Key6, 6),
            (KeyCode::Key7, 7),
            (KeyCode::Key8, 8),
            (KeyCode::Key9, 9),
            (KeyCode::Numpad0, 0),
            (KeyCode::Numpad1, 1),
            (KeyCode::Numpad2, 2),
            (KeyCode::Numpad3, 3),
            (KeyCode::Numpad4, 4),
            (KeyCode::Numpad5, 5),
            (KeyCode::Numpad6, 6),
            (KeyCode::Numpad7, 7),
            (KeyCode::Numpad8, 8),
            (KeyCode::Numpad9, 9),
        ] {
            if keyboard.just_released(*key) {
                number_field.current_value =
                    (number_field.current_value * 10 + value).min(number_field.max_value);
                break;
            }
        }

        if initial_value != number_field.current_value {
            if text.sections.len() == 0 {
                text.sections.push(TextSection {
                    value: format!("{}", number_field.current_value),
                    style: TextStyle {
                        font: Default::default(),
                        font_size: 14.0,
                        color: Color::BLACK,
                    },
                });
            } else {
                text.sections[0].value = format!("{}", number_field.current_value);
            }
            events.send(
                number_field
                    .event_generator
                    .create_event(number_field.current_value),
            );
        }
    });
}
