mod events;
mod rules_container;
mod state;

use bevy::prelude::Plugin;
pub use events::{
    ChangeViewTo, RuleUpdateEvent, RuleUpdateEventGenerator, ShowRulesFor, TogglePlay,
};
pub use rules_container::RulesContainer;
pub use state::{setup_menus, MenuState};

const HEADER_MARGIN: f32 = 20.0;
const HEADER_FONT_SIZE: f32 = 20.0;
const HEADER_HEIGHT: f32 = 50.0;
const REGULAR_MARGIN: f32 = 5.0;
const REGULAR_FONT_SIZE: f32 = 12.0;
const REGULAR_HEIGHT_STEP: f32 = 25.0;

pub struct MenusPlugin;

impl Plugin for MenusPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(MenuState::default())
            .add_startup_system(state::setup_menus)
            .add_system(events::change_view_to)
            .add_system(events::on_rule_update)
            .add_system(events::toggle_play_event)
            .add_system(rules_container::change_rules_event);
    }
}