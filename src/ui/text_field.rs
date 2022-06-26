
use bevy::{prelude::{Component, Query, EventWriter, Res, KeyCode, Color, EventReader}, text::{Text, TextSection, TextStyle}, input::Input, window::ReceivedCharacter};

use super::element::UiElement;

/// Trait for structs that can generate an event given a value.
pub trait TextEventGenerator {
    type Event: Component + Clone;
    fn create_event(&self, value: String) -> Self::Event;
}

/// Component for UI Elements that allows for typing strings.
#[derive(Component)]
pub struct TextField<EventGenerator: Component + TextEventGenerator> {
    /// A generator used to create the events when the value of the field is confirmed.
    pub event_generator: EventGenerator,
    pub current_value: String,
}



/// Detect button presses on selected text fields to type in letters on them or confirm the value
pub(super) fn text_field_handler<EventGenerator: Component + TextEventGenerator>(
    mut query: Query<(&mut Text, &mut TextField<EventGenerator>, &UiElement)>,
    mut events: EventWriter<EventGenerator::Event>,
    keyboard: Res<Input<KeyCode>>,
    mut char_event: EventReader<ReceivedCharacter>,
) {
    query.for_each_mut(|(mut text, mut text_field, element)| {
        if !element.selected_state.current {
            return;
        }
        let initial_value = text_field.current_value.clone();
        for char in char_event.iter() {
            if char.char == '\u{7f}' || char.char == '\u{08}' {
                text_field.current_value.pop();
            } else if !char.char.is_control() {
                text_field.current_value.push(char.char);
            }
        }

        if keyboard.just_released(KeyCode::NumpadEnter) || keyboard.just_released(KeyCode::Return) {
            let mut confirmed_string = String::new();
            std::mem::swap(&mut confirmed_string, &mut text_field.current_value);
            events.send(text_field.event_generator.create_event(confirmed_string));
        }

        if initial_value != text_field.current_value {
            if text.sections.len() == 0 {
                text.sections.push(TextSection {
                    value: text_field.current_value.clone(),
                    style: TextStyle {
                        font: Default::default(),
                        font_size: 14.0,
                        color: Color::BLACK,
                    },
                });
            } else {
                text.sections[0].value = text_field.current_value.clone();
            }
        }
    });
}