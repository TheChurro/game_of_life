use bevy::{
    hierarchy::Children,
    input::Input,
    math::{Size, Vec2, Vec3},
    prelude::{
        Added, Changed, Color, Component, Entity, EventReader, EventWriter, Handle, Image,
        KeyCode, Query, Res, Transform,
    },
    sprite::Sprite,
    text::{Text, TextSection, TextStyle},
    window::{WindowResized, Windows},
};

#[derive(Component)]
pub struct UiElement {
    pub size: Size,
    pub hover_state: UiStateDetails<bool>,
    pub click_state: UiStateDetails<bool>,
    pub selected_state: UiStateDetails<bool>,
    pub scroll_state: UiStateDetails<Vec2>,
}

#[derive(Component)]
pub struct AnchoredUi {
    pub x_percent: f32,
    pub y_percent: f32,
    pub width_grow: Option<f32>,
    pub height_grow: Option<f32>,
}

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
    pub fn exited(&self) -> bool {
        !self.current && self.previous
    }

    pub fn entered(&self) -> bool {
        self.current && !self.previous
    }
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

pub trait NumberedEventGenerator {
    type Event: Component;
    fn create_event(&self, value: u32) -> Self::Event;
}

#[derive(Component)]
pub struct NumberField<EventGenerator: Component + NumberedEventGenerator> {
    pub event_generator: EventGenerator,
    pub current_value: u32,
    pub max_value: u32,
    pub min_value: u32,
}

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

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

pub fn linear_scroll_handler(
    mut transform_query: Query<(&mut Transform, &UiElement)>,
    mut scroll_query: Query<
        (Entity, &mut UiLinearScroll, &UiElement, &Children),
        Changed<UiElement>,
    >,
) {
    scroll_query.for_each_mut(|(entity, mut scroll, element, children)| {
        if element.scroll_state.current.length_squared() > 0.0001 {
            scroll.scroll_position += element.scroll_state.current * Vec2::new(1.0, -1.0);

            let size = match transform_query.get(entity) {
                Ok((_, element)) => element.size.clone(),
                Err(_) => Size::new(0.0, 0.0),
            };

            position_scroll_children(children, &mut transform_query, size, &mut scroll);
        }
    });
}

pub fn update_sprite_to_match_layout(
    mut query: Query<(&mut Sprite, &UiElement), Changed<UiElement>>,
) {
    query.for_each_mut(|(mut sprite, element)| {
        sprite.custom_size = Some(Vec2::new(element.size.width, element.size.height));
    });
}

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
