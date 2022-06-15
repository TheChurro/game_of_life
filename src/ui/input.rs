use bevy::{
    hierarchy::{Children, Parent},
    input::{
        mouse::{MouseMotion, MouseWheel},
        Input,
    },
    math::{Vec2, Vec3Swizzles},
    prelude::{Entity, EventReader, MouseButton, Query, Res, Transform, With, Without},
    window::Windows,
};

use super::UiElement;

pub struct InputState {
    ui_element_clicked: Option<Entity>,
    ui_element_clicked_buffered: Option<Entity>,
    ui_element_selected: Option<Entity>,
    ui_element_selected_buffered: Option<Entity>,
    ui_element_scrolled: Option<Entity>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            ui_element_clicked: None,
            ui_element_clicked_buffered: None,
            ui_element_selected: None,
            ui_element_selected_buffered: None,
            ui_element_scrolled: None,
        }
    }
}

const SCROLL_SENSITIVITY: f32 = 0.5;

fn update_hovers(ui_element_query: &mut Query<(&Transform, &mut UiElement, Option<&Children>)>) {
    ui_element_query.for_each_mut(|(_, mut element, _)| {
        if element.hover_state.accepts_state && (element.hover_state.current || element.hover_state.previous) {
            element.hover_state.previous = element.hover_state.current;
            element.hover_state.current = false;
        }
    });
}

fn find_event_targets(
    entity: Entity,
    mut hover_position: Vec2,
    ui_element_query: &mut Query<(&Transform, &mut UiElement, Option<&Children>)>,
) -> (bool, Option<Entity>, Option<Entity>, Option<Entity>) {
    let mut is_hovered = false;

    let mut click_target = None;
    let mut scroll_target = None;
    let mut select_target = None;

    let children =
        if let Ok((transform, mut element, maybe_children)) = ui_element_query.get_mut(entity) {
            // Check to see if we are hovered (or we were hovered last frame).
            hover_position -= transform.translation.xy();

            is_hovered = hover_position.x.abs() <= element.size.width / 2.0
                && hover_position.y.abs() <= element.size.height / 2.0;

            // If this element can be hovered and hover has changed, update that state.
            if element.hover_state.accepts_state {
                if element.hover_state.current != is_hovered {
                    element.hover_state.current = is_hovered;
                }
            }

            if is_hovered {
                click_target = if element.click_state.accepts_state {
                    Some(entity)
                } else {
                    None
                };
                scroll_target = if element.scroll_state.accepts_state {
                    Some(entity)
                } else {
                    None
                };
                select_target = if element.selected_state.accepts_state {
                    Some(entity)
                } else {
                    None
                };
            }

            if let Some(children) = maybe_children {
                children.iter().cloned().collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

    if is_hovered {
        for child in children.iter().rev() {
            let (_, child_click_target, child_scroll_target, child_select_target) =
                find_event_targets(*child, hover_position, ui_element_query);
            click_target = child_click_target.or(click_target);
            scroll_target = child_scroll_target.or(scroll_target);
            select_target = child_select_target.or(select_target);
        }
    }

    (is_hovered, click_target, scroll_target, select_target)
}

pub struct ProcessedInputs {
    pub over_some_ui: bool,
    pub scroll: Vec2,
    pub movement: Vec2,
}

impl InputState {
    pub fn process_inputs(
        &mut self,
        mouse_input: &Res<Input<MouseButton>>,
        mut mouse_movements: EventReader<MouseMotion>,
        mut mouse_wheel_movements: EventReader<MouseWheel>,
        windows: &Res<Windows>,
        ui_roots_query: Query<Entity, (With<UiElement>, Without<Parent>)>,
        mut ui_element_query: Query<(&Transform, &mut UiElement, Option<&Children>)>,
    ) -> ProcessedInputs {
        let mut scroll = Vec2::ZERO;
        for motion in mouse_wheel_movements.iter() {
            scroll += Vec2::new(motion.x, motion.y) * SCROLL_SENSITIVITY;
        }

        let mut movement = Vec2::ZERO;
        for motion in mouse_movements.iter() {
            movement += motion.delta;
        }

        // Adjust the scroll for our last scrolled entity.
        if let Some(entity) = self.ui_element_scrolled {
            if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                element.scroll_state.previous = element.scroll_state.current;
                element.scroll_state.current = Vec2::ZERO;
            }
        }
        self.ui_element_scrolled = None;

        // Adjust the selected element state for the last selected entity.
        if self.ui_element_selected != self.ui_element_selected_buffered {
            if let Some(entity) = self.ui_element_selected_buffered {
                if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                    element.selected_state.previous = element.selected_state.current;
                    element.selected_state.current = false;
                }
            }
        }
        self.ui_element_selected_buffered = None;

        let mut clear_select = false;
        if let Some(entity) = self.ui_element_selected {
            clear_select = mouse_input.just_pressed(MouseButton::Left);
            if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                element.selected_state.previous = element.selected_state.current;
                element.selected_state.current = !clear_select;
            }
        }
        if clear_select {
            self.ui_element_selected_buffered = self.ui_element_selected;
            self.ui_element_selected = None;
        }

        // Adjust our click states
        if self.ui_element_clicked != self.ui_element_clicked_buffered {
            if let Some(entity) = self.ui_element_clicked_buffered {
                if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                    element.click_state.previous = element.click_state.current;
                    element.click_state.current = false;
                }
            }
        }
        self.ui_element_clicked_buffered = None;

        let mut clear_click = false;
        if let Some(entity) = self.ui_element_clicked {
            clear_click = !mouse_input.pressed(MouseButton::Left);
            if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                element.click_state.previous = element.click_state.current;
                if clear_click {
                    element.click_state.current = false;
                }
            }
        }
        if clear_click {
            self.ui_element_clicked_buffered = self.ui_element_clicked;
            self.ui_element_clicked = None;
        }

        // Go through and unhover things. If we are still hovering them, we will update that below.
        update_hovers(&mut ui_element_query);

        let mut over_ui = false;
        // If we have a mouse position, we are going to go issue hovers, clicks, selects and scrolls
        if let Some(mouse_position) = windows
            .get_primary()
            .and_then(|window| window.cursor_position())
        {
            let mouse_position = mouse_position
                - Vec2::new(windows.primary().width(), windows.primary().height()) * 0.5;
            let mut click_target = None;
            let mut scroll_target = None;
            let mut select_target = None;

            for root in ui_roots_query.iter() {
                let (is_hovered, root_click_target, root_scroll_target, root_select_target) =
                    find_event_targets(root, mouse_position, &mut ui_element_query);
                click_target = root_click_target.or(click_target);
                scroll_target = root_scroll_target.or(scroll_target);
                select_target = root_select_target.or(select_target);

                over_ui |= is_hovered;
            }

            if mouse_input.just_pressed(MouseButton::Left) {
                if let Some(entity) = click_target {
                    if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                        element.click_state.previous = element.click_state.current;
                        element.click_state.current = true;
                    }
                    self.ui_element_clicked = Some(entity);
                } else if let Some(entity) = select_target {
                    if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                        element.selected_state.previous = element.selected_state.current;
                        element.selected_state.current = true;
                    }
                    self.ui_element_selected = Some(entity);
                }
            } else if !mouse_input.pressed(MouseButton::Left) {
                if let Some(entity) = scroll_target {
                    if let Ok((_, mut element, _)) = ui_element_query.get_mut(entity) {
                        element.scroll_state.previous = element.scroll_state.current;
                        element.scroll_state.current = scroll;
                    }
                    self.ui_element_scrolled = Some(entity);
                }
            }
        };

        ProcessedInputs {
            over_some_ui: over_ui,
            scroll,
            movement,
        }
    }
}
