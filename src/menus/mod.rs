mod events;
mod rules_container;
mod state;
mod tile_inspect;

use bevy::prelude::{Plugin, ParallelSystemDescriptorCoercion};
pub use events::{
    ChangeViewTo, RuleUpdateEvent, RuleUpdateEventGenerator, ShowRulesFor, TogglePlay,
};
pub use rules_container::RulesContainer;
pub use state::{setup_menus, MenuState};
pub use tile_inspect::{DebugTileEvent, CommandEventGenerator, CommandEvent, DebugRoot, DebugState};

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
            .insert_resource(DebugState::default())
            .add_startup_system(state::setup_menus)
            .add_system(events::change_view_to)
            .add_system(events::on_rule_update)
            .add_system(events::toggle_play_event)
            .add_system(rules_container::change_rules_event)
            .add_system(tile_inspect::inspect)
            .add_system(tile_inspect::adjust_child_sizes.before(crate::ui::scroll_view::linear_scroll_handler))
            .add_system(tile_inspect::process_debug_inserts)
            .add_system(tile_inspect::update_debugger_panel)
            .add_system(tile_inspect::display_debug_options);
    }
}
