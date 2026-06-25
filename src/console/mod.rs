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
use wasserxr::{Uuid, attacher, detacher, scene::Scene, system};

const TABS: [&str; 4] = ["Entities", "Plugins", "Systems", "Log"];

static TERMINAL: LazyLock<Mutex<Option<DefaultTerminal>>> = LazyLock::new(|| Mutex::new(None));
static STATE: LazyLock<Mutex<Screen>> = LazyLock::new(|| Mutex::new(Screen::default()));

#[derive(Clone)]
enum Screen {
    EntityList(usize),
    EntityDetails(Uuid, usize),
    Prompt(TextPrompt),
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
        Self {
            title: title.into(),
            text: String::new(),
            offset: 0,
            on_submit,
            on_cancel: Box::new(on_cancel),
        }
    }
}

#[derive(Clone)]
enum PromptSubmit {
    RenameEntity(Uuid),
    CreateEntity,
}

impl PromptSubmit {
    fn run(self, scene: &mut Scene, text: String) -> Screen {
        match self {
            Self::RenameEntity(id) => {
                let _ = scene.set_entity_name(id, text);
                Screen::EntityDetails(id, 0)
            }
            Self::CreateEntity => {
                let entity = scene.add_entity();
                let _ = scene.set_entity_name(entity, text);
                Screen::EntityDetails(entity, 0)
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
fn console(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
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
            KeyCode::Char('r') => {
                let _ = scene.reload();
                Screen::EntityList(index)
            }
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
            _ => Screen::EntityDetails(id, component_index),
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
    (index + 1) % len
}

fn index_sub_no_loop(index: usize) -> usize {
    if index == 0 { 0 } else { index - 1 }
}

fn index_sub_with_loop(index: usize, len: usize) -> usize {
    if index == 0 { len - 1 } else { index - 1 }
}

fn draw(frame: &mut Frame, scene: &Scene, state: Screen) {
    match state {
        Screen::EntityList(index) => draw_entity_list(frame, scene, index),
        Screen::EntityDetails(id, component_index) => {
            draw_entity_detail(frame, scene, id, component_index)
        }
        Screen::Prompt(prompt) => draw_text_prompt(frame, &prompt),
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
