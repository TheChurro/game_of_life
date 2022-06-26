use bevy::prelude::{App, Component, ParallelSystemDescriptorCoercion, Plugin};

pub mod anchor;
pub mod button;
pub mod element;
pub mod input;
pub mod number_field;
pub mod text_field;
pub mod scroll_view;

pub use anchor::AnchoredUi;
pub use button::Button;
pub use element::{UiElement, UiStateDetails};
pub use input::InputState;
pub use number_field::{NumberField, NumberedEventGenerator};
pub use scroll_view::{LayoutDirection, UiLinearScroll};
pub use text_field::{TextEventGenerator, TextField};

pub struct UIPlugin {
    registry_functions: Vec<Box<dyn Fn(&mut App) + Sync + Send>>,
}

impl UIPlugin {
    pub fn new() -> Self {
        Self {
            registry_functions: Vec::new(),
        }
    }

    pub fn register_event<Evt: Component + Clone>(mut self) -> Self {
        self.registry_functions.push(Box::new(|app: &mut App| {
            app.add_event::<Evt>();
            app.add_system(button::button_handler::<Evt>);
        }));
        self
    }

    pub fn register_number_event_generator<EvtGen: NumberedEventGenerator + Component>(mut self) -> Self {
        self.registry_functions.push(Box::new(|app: &mut App| {
            app.add_system(number_field::number_field_handler::<EvtGen>);
        }));
        self.register_event::<EvtGen::Event>()
    }

    pub fn register_text_event_generator<EvtGen: TextEventGenerator + Component>(mut self) -> Self {
        self.registry_functions.push(Box::new(|app: &mut App| {
            app.add_system(text_field::text_field_handler::<EvtGen>);
        }));
        self.register_event::<EvtGen::Event>()
    }
}

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(InputState::default());
        app.add_system(element::update_text_to_match_layout);
        app.add_system(element::update_sprite_to_match_layout);
        app.add_system(anchor::position_on_added);
        app.add_system(anchor::position_on_window_changed);
        app.add_system(scroll_view::linear_scroll_children_changed);
        app.add_system(
            scroll_view::linear_scroll_handler.after(anchor::position_on_window_changed),
        );
        for func in &self.registry_functions {
            func(app);
        }
    }
}
