use std::{
    sync::{LazyLock, Mutex},
    time::Duration,
};

use crossterm::event::KeyCode;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Position, Rect},
    style::Color,
    symbols,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};
use wasserxr::{Uuid, attacher, detacher, error::SceneError, scene::Scene, system};

const TABS: [&str; 4] = ["Entities", "Plugins", "Systems", "Log"];

static TERMINAL: LazyLock<Mutex<Option<DefaultTerminal>>> = LazyLock::new(|| Mutex::new(None));
static STATE: LazyLock<Mutex<Screen>> = LazyLock::new(|| Mutex::new(Screen::default()));

#[derive(Clone)]
enum Screen {
    EntityList(usize),
    EntityDetails(Uuid, usize),
    ComponentDetails {
        entity_id: Uuid,
        component_id: String,
        component_index: usize,
        field_index: usize,
    },
    Prompt(TextPrompt),
    Error(ErrorScreen),
    PluginList(usize),
    SystemList(usize),
}

#[derive(Clone)]
struct TextPrompt {
    title: String,
    text: String,
    offset: usize,
    on_submit: PromptSubmit,
    on_cancel: Box<Screen>,
}

impl TextPrompt {
    fn new(title: impl Into<String>, on_submit: PromptSubmit, on_cancel: Screen) -> Self {
        Self::new_with_text(title, String::new(), on_submit, on_cancel)
    }

    fn new_with_text(
        title: impl Into<String>,
        text: String,
        on_submit: PromptSubmit,
        on_cancel: Screen,
    ) -> Self {
        let offset = text.len();
        Self {
            title: title.into(),
            text,
            offset,
            on_submit,
            on_cancel: Box::new(on_cancel),
        }
    }
}

#[derive(Clone)]
struct ErrorScreen {
    message: String,
    on_close: Box<Screen>,
}

impl ErrorScreen {
    fn new(message: impl Into<String>, on_close: Screen) -> Self {
        Self {
            message: message.into(),
            on_close: Box::new(on_close),
        }
    }
}

#[derive(Clone)]
enum PromptSubmit {
    RenameEntity(Uuid),
    CreateEntity,
    CreateComponent(Uuid),
    SetComponentField {
        entity_id: Uuid,
        component_id: String,
        field_id: String,
        component_index: usize,
        field_index: usize,
    },
}

impl PromptSubmit {
    fn run(self, scene: &mut Scene, text: String) -> Screen {
        match self {
            Self::RenameEntity(id) => match scene.set_entity_name(id, text) {
                Ok(()) => Screen::EntityDetails(id, 0),
                Err(error) => Screen::Error(ErrorScreen::new(
                    format!("Failed to rename entity {}: {:?}", id, error),
                    Screen::EntityDetails(id, 0),
                )),
            },
            Self::CreateEntity => {
                let entity = scene.add_entity();
                match scene.set_entity_name(entity, text) {
                    Ok(()) => Screen::EntityDetails(entity, 0),
                    Err(error) => Screen::Error(ErrorScreen::new(
                        format!("Failed to name entity {}: {:?}", entity, error),
                        Screen::EntityDetails(entity, 0),
                    )),
                }
            }
            Self::CreateComponent(entity_id) => {
                let component_id = text;
                match scene.add_component(entity_id, component_id.clone()) {
                    Ok(()) => Screen::ComponentDetails {
                        entity_id,
                        component_id,
                        component_index: 0,
                        field_index: 0,
                    },
                    Err(SceneError::EntityNotFound) => Screen::Error(ErrorScreen::new(
                        format!("Entity {} no longer exists: EntityNotFound", entity_id),
                        Screen::EntityDetails(entity_id, 0),
                    )),
                    Err(SceneError::ComponentCreation) => Screen::Error(ErrorScreen::new(
                        format!(
                            "Failed to create component {}: ComponentCreation",
                            component_id
                        ),
                        Screen::EntityDetails(entity_id, 0),
                    )),
                    Err(error) => Screen::Error(ErrorScreen::new(
                        format!("Failed to create component {}: {:?}", component_id, error),
                        Screen::EntityDetails(entity_id, 0),
                    )),
                }
            }
            Self::SetComponentField {
                entity_id,
                component_id,
                field_id,
                component_index,
                field_index,
            } => {
                let component_screen = Screen::ComponentDetails {
                    entity_id,
                    component_id: component_id.clone(),
                    component_index,
                    field_index,
                };
                match scene.parse_field(entity_id, &component_id, &field_id, &text) {
                    Ok(()) => component_screen,
                    Err(error) => Screen::Error(ErrorScreen::new(
                        format!("Failed to set {}.{}: {:?}", component_id, field_id, error),
                        component_screen,
                    )),
                }
            }
        }
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self::EntityList(0)
    }
}

#[system]
fn console(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    let Ok(mut state) = STATE.lock() else {
        return;
    };

    if let Some(key) = get_input() {
        *state = transition(scene, key, state.clone());
    }

    if let Ok(mut terminal) = TERMINAL.lock()
        && let Some(terminal) = terminal.as_mut()
    {
        let _ = terminal.draw(|frame| {
            draw(frame, scene, state.clone());
        });
    }
}

#[attacher(console)]
fn console_attacher(_scene: &mut Scene) {
    // Add the terminal
    if let Ok(mut terminal) = TERMINAL.lock() {
        *terminal = Some(ratatui::init());
    }
}

#[detacher(console)]
fn console_detacher(_scene: &mut Scene) {
    // Remove the terminal
    if let Ok(mut terminal) = TERMINAL.lock() {
        let _ = terminal.take();
    }
    ratatui::restore();
}

fn get_input() -> Option<KeyCode> {
    if let Ok(true) = crossterm::event::poll(Duration::from_secs(0)) {
        if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
            if key.kind == crossterm::event::KeyEventKind::Press {
                Some(key.code)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn transition(scene: &mut Scene, input: KeyCode, state: Screen) -> Screen {
    match state {
        // Entity List
        Screen::EntityList(index) => match input {
            KeyCode::Char('r') => match scene.reload() {
                Ok(()) => Screen::EntityList(index),
                Err(error) => Screen::Error(ErrorScreen::new(
                    format!("Failed to reload scene: {:?}", error),
                    Screen::EntityList(index),
                )),
            },
            KeyCode::Char('q') => {
                scene.should_exit();
                Screen::EntityList(index)
            }
            KeyCode::Char('h') | KeyCode::Left => Screen::SystemList(0),
            KeyCode::Char('l') | KeyCode::Right => Screen::PluginList(0),
            KeyCode::Down | KeyCode::Char('j') => {
                Screen::EntityList(index_add_with_loop(index, scene.get_entities().len()))
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Screen::EntityList(index_sub_with_loop(index, scene.get_entities().len()))
            }
            KeyCode::Enter => {
                if let Some(id) = scene.get_entities().get(index) {
                    Screen::EntityDetails(*id, 0)
                } else {
                    Screen::EntityList(index)
                }
            }
            KeyCode::Char('a') => Screen::Prompt(TextPrompt::new(
                "New Entity Name",
                PromptSubmit::CreateEntity,
                Screen::EntityList(index),
            )),
            KeyCode::Char('D') => {
                let entities = scene.get_entities();
                if entities.is_empty() {
                    Screen::EntityList(index)
                } else {
                    if let Some(id) = entities.get(index) {
                        match scene.remove_entity(*id) {
                            Ok(()) => {
                                Screen::EntityList(index.min(entities.len().saturating_sub(2)))
                            }
                            Err(error) => Screen::Error(ErrorScreen::new(
                                format!("Failed to delete entity {}: {:?}", id, error),
                                Screen::EntityList(index),
                            )),
                        }
                    } else {
                        Screen::EntityList(index)
                    }
                }
            }
            _ => Screen::EntityList(index),
        },
        // Entity Details
        Screen::EntityDetails(id, component_index) => match input {
            KeyCode::Esc => {
                if let Some(index) = scene.get_entities().iter().position(|x| id == *x) {
                    Screen::EntityList(index)
                } else {
                    Screen::EntityList(0)
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Ok(components) = scene.get_entity_components(id) {
                    Screen::EntityDetails(
                        id,
                        index_add_with_loop(component_index, components.len()),
                    )
                } else {
                    Screen::EntityDetails(id, component_index)
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Ok(components) = scene.get_entity_components(id) {
                    Screen::EntityDetails(
                        id,
                        index_sub_with_loop(component_index, components.len()),
                    )
                } else {
                    Screen::EntityDetails(id, component_index)
                }
            }
            KeyCode::Char('r') => Screen::Prompt(TextPrompt::new(
                format!("Rename Entity: {}", id),
                PromptSubmit::RenameEntity(id),
                Screen::EntityDetails(id, 0),
            )),
            KeyCode::Char('a') => Screen::Prompt(TextPrompt::new(
                "New Component Name",
                PromptSubmit::CreateComponent(id),
                Screen::EntityDetails(id, component_index),
            )),
            KeyCode::Enter => {
                if let Ok(components) = scene.get_entity_components(id)
                    && let Some(component_id) = components.get(component_index)
                {
                    Screen::ComponentDetails {
                        entity_id: id,
                        component_id: component_id.clone(),
                        component_index,
                        field_index: 0,
                    }
                } else {
                    Screen::EntityDetails(id, component_index)
                }
            }
            _ => Screen::EntityDetails(id, component_index),
        },
        Screen::ComponentDetails {
            entity_id,
            component_id,
            component_index,
            field_index,
        } => match input {
            KeyCode::Esc => Screen::EntityDetails(entity_id, component_index),
            KeyCode::Down | KeyCode::Char('j') => {
                if let Ok(fields) = scene.get_component_fields(entity_id, &component_id) {
                    Screen::ComponentDetails {
                        entity_id,
                        component_id,
                        component_index,
                        field_index: index_add_with_loop(field_index, fields.len()),
                    }
                } else {
                    Screen::ComponentDetails {
                        entity_id,
                        component_id,
                        component_index,
                        field_index,
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Ok(fields) = scene.get_component_fields(entity_id, &component_id) {
                    Screen::ComponentDetails {
                        entity_id,
                        component_id,
                        component_index,
                        field_index: index_sub_with_loop(field_index, fields.len()),
                    }
                } else {
                    Screen::ComponentDetails {
                        entity_id,
                        component_id,
                        component_index,
                        field_index,
                    }
                }
            }
            KeyCode::Enter => {
                if let Ok(fields) = scene.get_component_fields(entity_id, &component_id)
                    && let Some(field_id) = fields.get(field_index)
                {
                    let component_screen = Screen::ComponentDetails {
                        entity_id,
                        component_id: component_id.clone(),
                        component_index,
                        field_index,
                    };
                    match scene.render_field(entity_id, &component_id, field_id) {
                        Ok(current_value) => Screen::Prompt(TextPrompt::new_with_text(
                            format!("Set {}.{}", component_id, field_id),
                            current_value,
                            PromptSubmit::SetComponentField {
                                entity_id,
                                component_id,
                                field_id: field_id.clone(),
                                component_index,
                                field_index,
                            },
                            component_screen,
                        )),
                        Err(error) => Screen::Error(ErrorScreen::new(
                            format!("Failed to read {}.{}: {:?}", component_id, field_id, error),
                            component_screen,
                        )),
                    }
                } else {
                    Screen::ComponentDetails {
                        entity_id,
                        component_id,
                        component_index,
                        field_index,
                    }
                }
            }
            _ => Screen::ComponentDetails {
                entity_id,
                component_id,
                component_index,
                field_index,
            },
        },
        Screen::Prompt(mut prompt) => match input {
            KeyCode::Esc => *prompt.on_cancel,
            KeyCode::Backspace => {
                prompt.offset = index_sub_no_loop(prompt.offset);
                if prompt.offset < prompt.text.len() {
                    prompt.text.remove(prompt.offset);
                }
                Screen::Prompt(prompt)
            }
            KeyCode::Char(c) => {
                prompt.text.insert(prompt.offset, c);
                prompt.offset = index_add_no_loop(prompt.offset, prompt.text.len());
                Screen::Prompt(prompt)
            }
            KeyCode::Left => {
                prompt.offset = index_sub_no_loop(prompt.offset);
                Screen::Prompt(prompt)
            }
            KeyCode::Right => {
                prompt.offset = index_add_no_loop(prompt.offset, prompt.text.len());
                Screen::Prompt(prompt)
            }
            KeyCode::Enter => prompt.on_submit.run(scene, prompt.text),
            _ => Screen::Prompt(prompt),
        },
        Screen::Error(error) => match input {
            KeyCode::Char('q') | KeyCode::Enter | KeyCode::Esc => *error.on_close,
            _ => Screen::Error(error),
        },
        state => {
            // Global Keybinds
            state
        }
    }
}

fn index_add_no_loop(index: usize, len: usize) -> usize {
    index.saturating_add(1).min(len)
}

fn index_add_with_loop(index: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (index + 1) % len }
}

fn index_sub_no_loop(index: usize) -> usize {
    if index == 0 { 0 } else { index - 1 }
}

fn index_sub_with_loop(index: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else if index == 0 {
        len - 1
    } else {
        index - 1
    }
}

fn draw(frame: &mut Frame, scene: &Scene, state: Screen) {
    match state {
        Screen::EntityList(index) => draw_entity_list(frame, scene, index),
        Screen::EntityDetails(id, component_index) => {
            draw_entity_detail(frame, scene, id, component_index)
        }
        Screen::ComponentDetails {
            entity_id,
            component_id,
            component_index: _,
            field_index,
        } => draw_component_detail(frame, scene, entity_id, &component_id, field_index),
        Screen::Prompt(prompt) => draw_text_prompt(frame, &prompt),
        Screen::Error(error) => draw_error_screen(frame, &error),
        _ => build_place_holer_not_implemented(frame, frame.area()),
    }
}

fn draw_entity_list(frame: &mut Frame, scene: &Scene, index: usize) {
    let main_area = build_title_border(frame);
    let (header_area, content_area) = split_header_content(main_area);
    build_tab_header(frame, header_area, 0);

    let entity_ids = scene.get_entities();
    let entities: Vec<ListItem> = entity_ids
        .iter()
        .map(|id| {
            let name = scene
                .get_entity_name(*id)
                .map(ToString::to_string)
                .unwrap_or_else(|_| "Unnamed".to_string());

            ListItem::new(left_right_line(
                Span::raw(name),
                Span::styled(format!("{}", *id), Color::DarkGray),
                content_area.width,
            ))
        })
        .collect();

    let list = List::new(entities)
        .style(Color::White)
        .highlight_style(Color::Blue);

    let mut list_state = ListState::default();
    if !entity_ids.is_empty() {
        list_state.select(Some(index));
    }

    frame.render_stateful_widget(list, content_area, &mut list_state);
}

fn draw_entity_detail(frame: &mut Frame, scene: &Scene, id: Uuid, component_index: usize) {
    let main_area = build_title_border(frame);
    let (header_area, content_area) = split_header_content(main_area);
    build_tab_header(frame, header_area, 0);

    let Ok(entity_name) = scene.get_entity_name(id) else {
        let error = Paragraph::new("Failed to find entity")
            .style(Color::Red)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(error, content_area);
        return;
    };

    // Layout the Screen into two sections: Entity and Components
    let entity_area = Rect {
        x: content_area.x,
        y: content_area.y,
        width: content_area.width,
        height: content_area.height.min(3),
    };

    let component_area = Rect {
        x: content_area.x,
        y: content_area.y.saturating_add(entity_area.height),
        width: content_area.width,
        height: content_area.height.saturating_sub(entity_area.height),
    };

    // Draw the Entity Part
    let main_line = Paragraph::new(left_right_line(
        Span::raw(entity_name),
        Span::styled(format!("{}", id), Color::DarkGray),
        content_area.width,
    ))
    .block(Block::bordered().style(Color::White).title("Entity"));

    frame.render_widget(main_line, entity_area);

    // Component List
    let component_block = Block::bordered().style(Color::White).title("Components");
    let inner_component_block = component_block.inner(component_area);
    frame.render_widget(component_block, component_area);

    let Ok(components) = scene.get_entity_components(id) else {
        let error = Paragraph::new("Failed to find get components")
            .style(Color::Red)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(error, inner_component_block);
        return;
    };
    let mut list_state = ListState::default();
    if !components.is_empty() {
        list_state.select(Some(component_index));
    }
    let list = List::new(components)
        .style(Color::White)
        .highlight_style(Color::Blue);

    frame.render_stateful_widget(list, inner_component_block, &mut list_state);
}

fn draw_component_detail(
    frame: &mut Frame,
    scene: &Scene,
    entity_id: Uuid,
    component_id: &str,
    field_index: usize,
) {
    let main_area = build_title_border(frame);
    let (header_area, content_area) = split_header_content(main_area);
    build_tab_header(frame, header_area, 0);

    let component_header_area = Rect {
        x: content_area.x,
        y: content_area.y,
        width: content_area.width,
        height: content_area.height.min(3),
    };

    let fields_area = Rect {
        x: content_area.x,
        y: content_area.y.saturating_add(component_header_area.height),
        width: content_area.width,
        height: content_area
            .height
            .saturating_sub(component_header_area.height),
    };

    let plugin_id = scene
        .get_entity_component_plugin_id(entity_id, component_id)
        .unwrap_or("Unknown");
    let plugin_label = if plugin_id.is_empty() {
        "Static"
    } else {
        plugin_id
    };

    let component_line = Paragraph::new(left_right_line(
        Span::raw(component_id.to_owned()),
        Span::styled(format!("Plugin: {plugin_label}"), Color::DarkGray),
        content_area.width,
    ))
    .block(Block::bordered().style(Color::White).title("Component"));
    frame.render_widget(component_line, component_header_area);

    let fields_block = Block::bordered().style(Color::White).title("Fields");
    let inner_fields_block = fields_block.inner(fields_area);
    frame.render_widget(fields_block, fields_area);

    let Ok(fields) = scene.get_component_fields(entity_id, component_id) else {
        let error = Paragraph::new("Failed to get component fields")
            .style(Color::Red)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(error, inner_fields_block);
        return;
    };

    let field_items: Vec<ListItem> = fields
        .iter()
        .map(|field_id| {
            let field_type = scene
                .get_component_field_type(entity_id, component_id, field_id)
                .map(|field_type| format!("{field_type:?}"))
                .unwrap_or_else(|_| "Unknown".to_owned());
            let value = scene
                .render_field(entity_id, component_id, field_id)
                .unwrap_or_else(|error| format!("{error:?}"));

            ListItem::new(left_right_line(
                Span::raw(format!("{field_id} ({field_type})")),
                Span::styled(value, Color::DarkGray),
                inner_fields_block.width,
            ))
        })
        .collect();

    let mut list_state = ListState::default();
    if !field_items.is_empty() {
        list_state.select(Some(field_index.min(field_items.len().saturating_sub(1))));
    }

    let list = List::new(field_items)
        .style(Color::White)
        .highlight_style(Color::Blue);

    frame.render_stateful_widget(list, inner_fields_block, &mut list_state);
}

fn draw_text_prompt(frame: &mut Frame, prompt: &TextPrompt) {
    let main_area = build_title_border(frame);
    let input_area = centered_rect(main_area, main_area.width.min(60), main_area.height.min(3));

    let input = Paragraph::new(prompt.text.as_str())
        .block(
            Block::bordered()
                .style(Color::Blue)
                .title(prompt.title.as_str()),
        )
        .style(Color::White);
    frame.render_widget(input, input_area);
    frame.set_cursor_position(Position::new(
        input_area.x + 1 + (prompt.offset as u16).min(input_area.width.saturating_sub(2)),
        input_area.y + 1,
    ));
}

fn draw_error_screen(frame: &mut Frame, error: &ErrorScreen) {
    let main_area = build_title_border(frame);
    let error_area = centered_rect(main_area, main_area.width.min(60), main_area.height.min(3));

    let message = Paragraph::new(error.message.as_str())
        .block(Block::bordered().style(Color::Red).title("Error"))
        .style(Color::Red)
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(message, error_area);
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn build_title_border(frame: &mut Frame) -> Rect {
    let title = Block::bordered()
        .title("WasserXR Console")
        .style(Color::Blue);
    let inner_area = title.inner(frame.area());
    frame.render_widget(title, frame.area());
    inner_area
}

fn split_header_content(area: Rect) -> (Rect, Rect) {
    let tab_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height.min(1),
    };

    let content_area = Rect {
        x: area.x,
        y: area.y.saturating_add(tab_area.height),
        width: area.width,
        height: area.height.saturating_sub(tab_area.height),
    };

    (tab_area, content_area)
}

fn build_tab_header(frame: &mut Frame, header: Rect, index: usize) {
    let tab = ratatui::widgets::Tabs::new(TABS)
        .style(Color::White)
        .highlight_style(Color::Blue)
        .select(index)
        .divider(symbols::DOT)
        .padding(" ", " ");
    frame.render_widget(tab, header);
}

fn left_right_line<'a>(
    left: impl Into<Span<'a>>,
    right: impl Into<Span<'a>>,
    width: u16,
) -> Line<'a> {
    let left = left.into();
    let right = right.into();

    let left_width = left.width();
    let right_width = right.width();

    let spacing = width as usize;
    let spacing = spacing.saturating_sub(left_width + right_width);

    Line::from(vec![left, Span::raw(" ".repeat(spacing)), right])
}

fn build_place_holer_not_implemented(frame: &mut Frame, area: Rect) {
    let text = Paragraph::new("Not implemented!")
        .style(Color::Red)
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(text, area);
}
