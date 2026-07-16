use std::{cell::RefCell, io::Read, os::fd::AsRawFd};

use crossterm::event::KeyCode;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Position, Rect},
    style::Color,
    symbols,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};
use wasserxr::{
    Uuid,
    error::{ComponentError, PluginError, SceneError},
    scene::{
        Scene,
        logging::{LogEntry, LogLevel},
    },
    system,
    utils::paths::get_asset_path,
};

const TABS: [&str; 4] = ["Entities", "Plugins", "Systems", "Log"];
const LOG_TABS: [&str; 4] = ["DEBUG", "INFO", "WARN", "ERROR"];
const CONSOLE_RESOURCE: &str = "console_data";

struct ConsoleResource {
    terminal: DefaultTerminal,
    state: Screen,
}

impl ConsoleResource {
    fn new() -> Self {
        Self {
            terminal: ratatui::init(),
            state: Screen::default(),
        }
    }

    fn draw<F>(&mut self, render_callback: F)
    where
        F: FnOnce(&mut Frame),
    {
        let _ = self.terminal.draw(render_callback);
    }

    fn area(&self) -> Rect {
        self.terminal
            .size()
            .map(|size| Rect {
                x: 0,
                y: 0,
                width: size.width,
                height: size.height,
            })
            .unwrap_or_default()
    }
}

impl Drop for ConsoleResource {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

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
    SystemList {
        index: usize,
        error: Option<String>,
    },
    LogList {
        level: LogLevel,
        scroll: usize,
        follow: bool,
    },
}

#[derive(Clone, Copy)]
// Input handling needs layout facts, but drawing must not mutate state. This is to keep a clean MVC
// design pattern
struct ConsoleInputContext {
    visible_log_lines: u16,
}

impl ConsoleInputContext {
    fn new(area: Rect) -> Self {
        Self {
            // Log scrolling stops at the last visible page, so transition needs this bound.
            visible_log_lines: if can_draw_console(area) {
                log_area_height(area)
            } else {
                0
            },
        }
    }
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

fn scene_error_message(error: &SceneError) -> String {
    match error {
        SceneError::EntityNotFound => "the entity no longer exists".to_owned(),
        SceneError::ComponentAlreadyExists => {
            "a component with that name already exists on this entity".to_owned()
        }
        SceneError::SystemAlreadyExists => "a system with that name already exists".to_owned(),
        SceneError::PluginAlreadyLoaded => "the plugin is already loaded".to_owned(),
        SceneError::SystemNotFound => "the system was not found".to_owned(),
        SceneError::PluginNotFound => "the plugin was not found".to_owned(),
        SceneError::StaticPluginUnload => {
            "the built-in static plugin cannot be unloaded".to_owned()
        }
        SceneError::ComponentNotFound => "the component was not found on this entity".to_owned(),
        SceneError::ComponentFieldError(error) => component_error_reason(error),
        SceneError::PluginLoading(error) => plugin_error_reason(error),
        SceneError::SystemCreation => {
            "no loaded plugin could create that system; check the system name and exports"
                .to_owned()
        }
        SceneError::ComponentCreation => {
            "no loaded plugin could create that component; check the component name and exports"
                .to_owned()
        }
        SceneError::Serialization(message) => format!("scene serialization failed: {message}"),
        SceneError::Deserialization(message) => {
            format!("scene deserialization failed: {message}")
        }
        SceneError::FileIo(message) => format!("file operation failed: {message}"),
        _ => "Unknown Error".to_string(),
    }
}

fn component_error_reason(error: &ComponentError) -> String {
    match error {
        ComponentError::FieldNotFound => "the field was not found on this component".to_owned(),
        ComponentError::FieldNoGetter => {
            "the field cannot be read because it has no getter".to_owned()
        }
        ComponentError::FieldNotMutable => "the field is read-only".to_owned(),
        ComponentError::FieldNoSerializer => {
            "the field cannot be serialized because it has no serializer".to_owned()
        }
        ComponentError::FieldNoDeserializer => {
            "the field cannot be deserialized because it has no deserializer".to_owned()
        }
        ComponentError::NoCreator(error) => {
            format!(
                "the component creator is unavailable: {}",
                plugin_error_reason(error)
            )
        }
        ComponentError::NoDestroyer(error) => {
            format!(
                "the component destroyer is unavailable: {}",
                plugin_error_reason(error)
            )
        }
        ComponentError::FieldParsing => "the requested field list is invalid".to_owned(),
        ComponentError::FieldValueParsing => {
            "the entered value is not valid for this field type".to_owned()
        } // _ => "Unknown Error".to_string(),
    }
}

fn plugin_error_reason(error: &PluginError) -> String {
    match error {
        PluginError::LinkingError(message) => format!("linking failed: {message}"),
        PluginError::MissingSymbol(symbol) => {
            format!("the plugin is missing required symbol `{symbol}`")
        }
        PluginError::InvalidSymbol => "a symbol name contains an invalid null byte".to_owned(),
        _ => "Unknown Error".to_string(),
    }
}

#[derive(Clone)]
enum PromptSubmit {
    RenameEntity(Uuid),
    CreateEntity,
    CreatePlugin,
    SaveScene(Box<Screen>),
    ImportScene(Box<Screen>),
    CreateSystemId,
    CreateSystemPriority {
        system_id: String,
    },
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
                    scene_error_message(&error),
                    Screen::EntityDetails(id, 0),
                )),
            },
            Self::CreateEntity => {
                let entity = scene.add_entity();
                match scene.set_entity_name(entity, text) {
                    Ok(()) => Screen::EntityDetails(entity, 0),
                    Err(error) => Screen::Error(ErrorScreen::new(
                        scene_error_message(&error),
                        Screen::EntityDetails(entity, 0),
                    )),
                }
            }
            Self::CreatePlugin => {
                let Some(path) = get_asset_path(&text) else {
                    return Screen::Error(ErrorScreen::new(
                        format!("plugin path `{text}` was not found"),
                        Screen::PluginList(0),
                    ));
                };

                match scene.load_plugin(path.to_string_lossy().into_owned()) {
                    Ok(()) => Screen::PluginList(0),
                    Err(error) => Screen::Error(ErrorScreen::new(
                        scene_error_message(&error),
                        Screen::PluginList(0),
                    )),
                }
            }
            Self::SaveScene(on_done) => match scene.save(text) {
                Ok(()) => *on_done,
                Err(error) => {
                    Screen::Error(ErrorScreen::new(scene_error_message(&error), *on_done))
                }
            },
            Self::ImportScene(on_done) => {
                let Some(path) = get_asset_path(&text) else {
                    return Screen::Error(ErrorScreen::new(
                        format!("scene path `{text}` was not found"),
                        *on_done,
                    ));
                };

                match scene.load(path) {
                    Ok(()) => *on_done,
                    Err(error) => {
                        Screen::Error(ErrorScreen::new(scene_error_message(&error), *on_done))
                    }
                }
            }
            Self::CreateSystemId => Screen::Prompt(TextPrompt::new(
                "System Priority",
                PromptSubmit::CreateSystemPriority { system_id: text },
                Screen::SystemList {
                    index: 0,
                    error: None,
                },
            )),
            Self::CreateSystemPriority { system_id } => match text.parse::<usize>() {
                Ok(priority) => match scene.add_system(system_id, priority) {
                    Ok(()) => Screen::SystemList {
                        index: 0,
                        error: None,
                    },
                    Err(error) => Screen::Error(ErrorScreen::new(
                        scene_error_message(&error),
                        Screen::SystemList {
                            index: 0,
                            error: None,
                        },
                    )),
                },
                Err(_) => Screen::SystemList {
                    index: 0,
                    error: Some("System priority must be a usize".to_owned()),
                },
            },
            Self::CreateComponent(entity_id) => {
                let component_id = text;
                match scene.add_component(entity_id, component_id.clone()) {
                    Ok(()) => Screen::ComponentDetails {
                        entity_id,
                        component_id,
                        component_index: 0,
                        field_index: 0,
                    },
                    Err(error) => Screen::Error(ErrorScreen::new(
                        scene_error_message(&error),
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
                        scene_error_message(&error),
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
    ensure_console(scene);

    let (mut state, area) = {
        let Ok(console) = scene.get_resource::<RefCell<ConsoleResource>>(CONSOLE_RESOURCE) else {
            return;
        };
        let console = console.borrow();
        (console.state.clone(), console.area())
    };

    if let Some(key) = get_input() {
        state = transition(scene, key, ConsoleInputContext::new(area), state);
    }

    let Ok(console) = scene.get_resource::<RefCell<ConsoleResource>>(CONSOLE_RESOURCE) else {
        return;
    };

    let mut console = console.borrow_mut();
    console.state = state.clone();
    console.draw(|frame| {
        draw(frame, scene, state.clone());
    });
}

fn ensure_console(scene: &mut Scene) {
    if scene
        .get_resource::<RefCell<ConsoleResource>>(CONSOLE_RESOURCE)
        .is_err()
    {
        let _ = scene.add_resource(
            CONSOLE_RESOURCE.to_owned(),
            RefCell::new(ConsoleResource::new()),
        );
    }
}

fn get_input() -> Option<KeyCode> {
    // Do not use crossterm::event::poll/read here. On Unix, Crossterm lazily
    // registers a process-wide SIGWINCH handler for resize events and assumes
    // that handler lives for the whole process. This console is hot-reloaded as
    // a cdylib, so that handler can outlive the unloaded plugin code and crash
    // the next time the terminal is resized.
    let mut stdin = std::io::stdin();

    // PollFd mirrors libc's `struct pollfd`: `events` asks for readable stdin,
    // and libc's `poll` fills `revents`; timeout 0 keeps the ECS tick nonblocking.
    let mut poll_fd = libc::pollfd {
        fd: stdin.as_raw_fd(),
        events: 1,
        revents: 0,
    };

    let ready = unsafe { libc::poll(&mut poll_fd, 1, 0) };
    // `ready <= 0` means error/no input; missing 1 means stdin is not readable.
    if ready <= 0 || poll_fd.revents & 1 == 0 {
        return None;
    }

    // The terminal is in raw mode, so keys arrive as bytes. Read a small chunk:
    // enough for the escape sequences this console handles.
    let mut bytes = [0; 8];
    let len = stdin.read(&mut bytes).ok()?;
    parse_key(&bytes[..len])
}

fn parse_key(bytes: &[u8]) -> Option<KeyCode> {
    match bytes {
        [] => None,
        [b'\r' | b'\n', ..] => Some(KeyCode::Enter),
        [0x7f | 0x08, ..] => Some(KeyCode::Backspace),
        [0x1b, b'[', b'A', ..] => Some(KeyCode::Up),
        [0x1b, b'[', b'B', ..] => Some(KeyCode::Down),
        [0x1b, b'[', b'C', ..] => Some(KeyCode::Right),
        [0x1b, b'[', b'D', ..] => Some(KeyCode::Left),
        [0x1b, ..] => Some(KeyCode::Esc),
        [byte, ..] if byte.is_ascii() && !byte.is_ascii_control() => {
            Some(KeyCode::Char(*byte as char))
        }
        bytes => std::str::from_utf8(bytes)
            .ok()
            .and_then(|text| text.chars().next())
            .map(KeyCode::Char),
    }
}

fn transition(
    scene: &mut Scene,
    input: KeyCode,
    context: ConsoleInputContext,
    state: Screen,
) -> Screen {
    match state {
        // Entity List
        Screen::EntityList(index) => match input {
            KeyCode::Char('r') => match scene.reload() {
                Ok(()) => Screen::EntityList(index),
                Err(error) => Screen::Error(ErrorScreen::new(
                    scene_error_message(&error),
                    Screen::EntityList(index),
                )),
            },
            KeyCode::Char('q') => {
                scene.should_exit();
                Screen::EntityList(index)
            }
            KeyCode::Char('h') | KeyCode::Left => Screen::LogList {
                level: LogLevel::DEBUG,
                scroll: 0,
                follow: true,
            },
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
            KeyCode::Char('s') => scene_save_prompt(Screen::EntityList(index)),
            KeyCode::Char('i') => scene_import_prompt(Screen::EntityList(index)),
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
                                scene_error_message(&error),
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
            KeyCode::Char('D') => match scene.get_entity_components(id) {
                Ok(components) if components.is_empty() => {
                    Screen::EntityDetails(id, component_index)
                }
                Ok(components) => {
                    if let Some(component_id) = components.get(component_index) {
                        match scene.remove_component(id, component_id) {
                            Ok(()) => Screen::EntityDetails(
                                id,
                                component_index.min(components.len().saturating_sub(2)),
                            ),
                            Err(error) => Screen::Error(ErrorScreen::new(
                                scene_error_message(&error),
                                Screen::EntityDetails(id, component_index),
                            )),
                        }
                    } else {
                        Screen::EntityDetails(id, component_index)
                    }
                }
                Err(error) => Screen::Error(ErrorScreen::new(
                    scene_error_message(&error),
                    Screen::EntityDetails(id, component_index),
                )),
            },
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
                    let is_mutable = scene
                        .is_component_field_mutable(entity_id, &component_id, field_id)
                        .unwrap_or(false);
                    let is_string_parsable = scene
                        .is_component_field_string_parsable(entity_id, &component_id, field_id)
                        .unwrap_or(false);
                    if !(is_mutable && is_string_parsable) {
                        return component_screen;
                    }

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
                            scene_error_message(&error),
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
        Screen::PluginList(index) => match input {
            KeyCode::Char('r') => match scene.reload() {
                Ok(()) => Screen::PluginList(index),
                Err(error) => Screen::Error(ErrorScreen::new(
                    scene_error_message(&error),
                    Screen::PluginList(index),
                )),
            },
            KeyCode::Char('q') => {
                scene.should_exit();
                Screen::PluginList(index)
            }
            KeyCode::Char('h') | KeyCode::Left => Screen::EntityList(0),
            KeyCode::Char('l') | KeyCode::Right => Screen::SystemList {
                index: 0,
                error: None,
            },
            KeyCode::Down | KeyCode::Char('j') => {
                Screen::PluginList(index_add_with_loop(index, plugin_items(scene).len()))
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Screen::PluginList(index_sub_with_loop(index, plugin_items(scene).len()))
            }
            KeyCode::Char('a') => Screen::Prompt(TextPrompt::new(
                "Plugin Path",
                PromptSubmit::CreatePlugin,
                Screen::PluginList(index),
            )),
            KeyCode::Char('s') => scene_save_prompt(Screen::PluginList(index)),
            KeyCode::Char('i') => scene_import_prompt(Screen::PluginList(index)),
            KeyCode::Char('D') => {
                let plugins = plugin_items(scene);
                if let Some((_, plugin_id)) = plugins.get(index) {
                    match scene.unload_plugin(plugin_id) {
                        Ok(()) => Screen::PluginList(index.min(plugins.len().saturating_sub(2))),
                        Err(error) => Screen::Error(ErrorScreen::new(
                            scene_error_message(&error),
                            Screen::PluginList(index),
                        )),
                    }
                } else {
                    Screen::PluginList(index)
                }
            }
            _ => Screen::PluginList(index),
        },
        Screen::SystemList { index, error } => match input {
            KeyCode::Char('r') => match scene.reload() {
                Ok(()) => Screen::SystemList { index, error },
                Err(error) => Screen::Error(ErrorScreen::new(
                    scene_error_message(&error),
                    Screen::SystemList { index, error: None },
                )),
            },
            KeyCode::Char('q') => {
                scene.should_exit();
                Screen::SystemList { index, error }
            }
            KeyCode::Char('h') | KeyCode::Left => Screen::PluginList(0),
            KeyCode::Char('l') | KeyCode::Right => Screen::LogList {
                level: LogLevel::DEBUG,
                scroll: 0,
                follow: true,
            },
            KeyCode::Down | KeyCode::Char('j') => Screen::SystemList {
                index: index_add_with_loop(index, scene.get_systems().len()),
                error: None,
            },
            KeyCode::Up | KeyCode::Char('k') => Screen::SystemList {
                index: index_sub_with_loop(index, scene.get_systems().len()),
                error: None,
            },
            KeyCode::Char('a') => Screen::Prompt(TextPrompt::new(
                "System ID",
                PromptSubmit::CreateSystemId,
                Screen::SystemList { index, error: None },
            )),
            KeyCode::Char('s') => scene_save_prompt(Screen::SystemList { index, error: None }),
            KeyCode::Char('i') => scene_import_prompt(Screen::SystemList { index, error: None }),
            KeyCode::Char('D') => {
                let systems = scene.get_systems();
                if let Some(system_id) = systems.get(index) {
                    match scene.remove_system(system_id) {
                        Ok(()) => Screen::SystemList {
                            index: index.min(systems.len().saturating_sub(2)),
                            error: None,
                        },
                        Err(error) => Screen::Error(ErrorScreen::new(
                            scene_error_message(&error),
                            Screen::SystemList { index, error: None },
                        )),
                    }
                } else {
                    Screen::SystemList { index, error }
                }
            }
            _ => Screen::SystemList { index, error },
        },
        Screen::LogList {
            level,
            scroll,
            follow,
        } => match input {
            KeyCode::Char('q') => {
                scene.should_exit();
                Screen::LogList {
                    level,
                    scroll,
                    follow,
                }
            }
            KeyCode::Char('h') => Screen::SystemList {
                index: 0,
                error: None,
            },
            KeyCode::Char('l') => Screen::EntityList(0),
            KeyCode::Left => Screen::LogList {
                level: log_level_sub(level),
                scroll: 0,
                follow,
            },
            KeyCode::Right => Screen::LogList {
                level: log_level_add(level),
                scroll: 0,
                follow,
            },
            KeyCode::Char('f') => Screen::LogList {
                level,
                scroll,
                follow: true,
            },
            KeyCode::Down | KeyCode::Char('j') => {
                let log_count = filtered_logs(scene, level).len();
                let max_scroll = max_log_scroll(log_count, context.visible_log_lines);
                let scroll = if follow {
                    max_scroll
                } else {
                    clamp_log_scroll(scroll, log_count, context.visible_log_lines)
                };
                Screen::LogList {
                    level,
                    scroll: index_add_no_loop(scroll, max_scroll),
                    follow: false,
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let log_count = filtered_logs(scene, level).len();
                let max_scroll = max_log_scroll(log_count, context.visible_log_lines);
                let scroll = if follow {
                    max_scroll
                } else {
                    clamp_log_scroll(scroll, log_count, context.visible_log_lines)
                };
                Screen::LogList {
                    level,
                    scroll: index_sub_no_loop(scroll),
                    follow: false,
                }
            }
            _ => Screen::LogList {
                level,
                scroll,
                follow,
            },
        },
        Screen::Prompt(mut prompt) => match input {
            KeyCode::Esc => *prompt.on_cancel,
            KeyCode::Backspace => {
                if prompt.offset > 0 {
                    prompt.offset = index_sub_no_loop(prompt.offset);
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
    }
}

fn scene_save_prompt(on_done: Screen) -> Screen {
    Screen::Prompt(TextPrompt::new(
        "Save Scene Path",
        PromptSubmit::SaveScene(Box::new(on_done.clone())),
        on_done,
    ))
}

fn scene_import_prompt(on_done: Screen) -> Screen {
    Screen::Prompt(TextPrompt::new(
        "Import Scene Path",
        PromptSubmit::ImportScene(Box::new(on_done.clone())),
        on_done,
    ))
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

fn log_area_height(area: Rect) -> u16 {
    let main_area = Block::bordered().inner(area);
    let (_, content_area, _) = split_header_content_footer(main_area);
    let (_, log_area) = split_header_content(content_area);

    log_area.height
}

fn max_log_scroll(log_count: usize, visible_lines: u16) -> usize {
    log_count.saturating_sub(visible_lines.max(1) as usize)
}

fn clamp_log_scroll(scroll: usize, log_count: usize, visible_lines: u16) -> usize {
    scroll.min(max_log_scroll(log_count, visible_lines))
}

fn draw(frame: &mut Frame, scene: &Scene, state: Screen) {
    if !can_draw_console(frame.area()) {
        return;
    }

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
        Screen::PluginList(index) => draw_plugin_list(frame, scene, index),
        Screen::SystemList { index, error } => draw_system_list(frame, scene, index, error),
        Screen::LogList {
            level,
            scroll,
            follow,
        } => draw_log_list(frame, scene, level, scroll, follow),
        Screen::Prompt(prompt) => draw_text_prompt(frame, &prompt),
        Screen::Error(error) => draw_error_screen(frame, &error),
    }
}

fn can_draw_console(area: Rect) -> bool {
    area.width >= 2 && area.height >= 2
}

fn draw_entity_list(frame: &mut Frame, scene: &Scene, index: usize) {
    let main_area = build_title_border(frame);
    let (header_area, content_area, hint_area) = split_header_content_footer(main_area);
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
    draw_keymap_hint(
        frame,
        hint_area,
        "h/l tabs  j/k move  Enter open  a add  D delete  r reload  s save  i import  q quit",
    );
}

fn plugin_items(scene: &Scene) -> Vec<(String, String)> {
    let mut plugins = vec![("Statically Linked".to_owned(), String::new())];
    plugins.extend(
        scene
            .get_plugins()
            .into_iter()
            .map(|plugin| (plugin.clone(), plugin)),
    );
    plugins
}

fn plugin_label(plugin_id: &str) -> String {
    if plugin_id.is_empty() {
        "Statically Linked".to_owned()
    } else {
        plugin_id.to_owned()
    }
}

fn draw_plugin_list(frame: &mut Frame, scene: &Scene, index: usize) {
    let main_area = build_title_border(frame);
    let (header_area, content_area, hint_area) = split_header_content_footer(main_area);
    build_tab_header(frame, header_area, 1);

    let plugins = plugin_items(scene);
    let items: Vec<ListItem> = plugins
        .iter()
        .map(|(display, _)| ListItem::new(display.clone()))
        .collect();

    let mut list_state = ListState::default();
    if !items.is_empty() {
        list_state.select(Some(index.min(items.len().saturating_sub(1))));
    }

    let list = List::new(items)
        .style(Color::White)
        .highlight_style(Color::Blue);

    frame.render_stateful_widget(list, content_area, &mut list_state);
    draw_keymap_hint(
        frame,
        hint_area,
        "h/l tabs  j/k move  a add  D delete  r reload  s save  i import  q quit",
    );
}

fn draw_system_list(frame: &mut Frame, scene: &Scene, index: usize, error: Option<String>) {
    let main_area = build_title_border(frame);
    let (header_area, mut content_area, hint_area) = split_header_content_footer(main_area);
    build_tab_header(frame, header_area, 2);

    if let Some(error) = error {
        let error_area = Rect {
            x: content_area.x,
            y: content_area.y,
            width: content_area.width,
            height: content_area.height.min(1),
        };
        let error_text = Paragraph::new(error).style(Color::Red);
        frame.render_widget(error_text, error_area);
        content_area.y = content_area.y.saturating_add(error_area.height);
        content_area.height = content_area.height.saturating_sub(error_area.height);
    }

    let systems = scene.get_systems();
    let items: Vec<ListItem> = systems
        .iter()
        .map(|system_id| {
            let plugin_id = scene
                .get_system_plugin_id(system_id)
                .map(plugin_label)
                .unwrap_or_else(|_| "Unknown".to_owned());

            ListItem::new(left_right_line(
                Span::raw(system_id.clone()),
                Span::styled(plugin_id, Color::DarkGray),
                content_area.width,
            ))
        })
        .collect();

    let mut list_state = ListState::default();
    if !items.is_empty() {
        list_state.select(Some(index.min(items.len().saturating_sub(1))));
    }

    let list = List::new(items)
        .style(Color::White)
        .highlight_style(Color::Blue);

    frame.render_stateful_widget(list, content_area, &mut list_state);
    draw_keymap_hint(
        frame,
        hint_area,
        "h/l tabs  j/k move  a add  D delete  r reload  s save  i import  q quit",
    );
}

fn draw_log_list(frame: &mut Frame, scene: &Scene, level: LogLevel, scroll: usize, follow: bool) {
    let main_area = build_title_border(frame);
    let (header_area, content_area, hint_area) = split_header_content_footer(main_area);
    build_tab_header(frame, header_area, 3);

    let (level_area, log_area) = split_header_content(content_area);
    build_log_tab_header(frame, level_area, log_level_index(level), follow);

    let logs = filtered_logs(scene, level);
    let lines: Vec<Line> = logs
        .iter()
        .map(|entry| {
            Line::from(Span::styled(
                format!("{entry}"),
                log_color(entry.get_level()),
            ))
        })
        .collect();

    let visible_start = if follow {
        max_log_scroll(lines.len(), log_area.height)
    } else {
        clamp_log_scroll(scroll, lines.len(), log_area.height)
    };
    let list = Paragraph::new(lines).scroll((visible_start as u16, 0));
    frame.render_widget(list, log_area);
    draw_keymap_hint(
        frame,
        hint_area,
        "h/l tabs  Left/Right level  j/k scroll  f follow  q quit",
    );
}

fn filtered_logs(scene: &Scene, level: LogLevel) -> Vec<LogEntry> {
    scene
        .iter_logs()
        .filter(|entry| log_level_index(entry.get_level()) >= log_level_index(level))
        .collect()
}

fn log_level_add(level: LogLevel) -> LogLevel {
    match level {
        LogLevel::DEBUG => LogLevel::INFO,
        LogLevel::INFO => LogLevel::WARN,
        LogLevel::WARN | LogLevel::ERROR => LogLevel::ERROR,
    }
}

fn log_level_sub(level: LogLevel) -> LogLevel {
    match level {
        LogLevel::DEBUG | LogLevel::INFO => LogLevel::DEBUG,
        LogLevel::WARN => LogLevel::INFO,
        LogLevel::ERROR => LogLevel::WARN,
    }
}

fn log_level_index(level: LogLevel) -> usize {
    match level {
        LogLevel::DEBUG => 0,
        LogLevel::INFO => 1,
        LogLevel::WARN => 2,
        LogLevel::ERROR => 3,
    }
}

fn log_color(level: LogLevel) -> Color {
    match level {
        LogLevel::DEBUG => Color::Blue,
        LogLevel::INFO => Color::White,
        LogLevel::WARN => Color::Yellow,
        LogLevel::ERROR => Color::Red,
    }
}

fn draw_entity_detail(frame: &mut Frame, scene: &Scene, id: Uuid, component_index: usize) {
    let main_area = build_title_border(frame);
    let (header_area, content_area, hint_area) = split_header_content_footer(main_area);
    build_tab_header(frame, header_area, 0);

    let Ok(entity_name) = scene.get_entity_name(id) else {
        let error = Paragraph::new("Failed to find entity")
            .style(Color::Red)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(error, content_area);
        draw_keymap_hint(frame, hint_area, "Esc back");
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
        draw_keymap_hint(frame, hint_area, "Esc back");
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
    draw_keymap_hint(
        frame,
        hint_area,
        "Esc back  j/k move  Enter open  a add  r rename  D delete",
    );
}

fn draw_component_detail(
    frame: &mut Frame,
    scene: &Scene,
    entity_id: Uuid,
    component_id: &str,
    field_index: usize,
) {
    let main_area = build_title_border(frame);
    let (header_area, content_area, hint_area) = split_header_content_footer(main_area);
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
        draw_keymap_hint(frame, hint_area, "Esc back");
        return;
    };

    let field_items: Vec<ListItem> = fields
        .iter()
        .map(|field_id| {
            let field_type = scene.get_component_field_type(entity_id, component_id, field_id);
            let is_mutable = scene
                .is_component_field_mutable(entity_id, component_id, field_id)
                .unwrap_or(false);
            let is_string_parsable = scene
                .is_component_field_string_parsable(entity_id, component_id, field_id)
                .unwrap_or(false);
            let field_type = field_type
                .map(|field_type| format!("{field_type:?}"))
                .unwrap_or_else(|_| "Unknown".to_owned());
            let value = scene
                .render_field(entity_id, component_id, field_id)
                .unwrap_or_else(|error| format!("{error:?}"));
            let field_color = if is_mutable && is_string_parsable {
                Color::White
            } else {
                Color::Red
            };

            ListItem::new(left_right_line(
                Span::styled(format!("{field_id} ({field_type})"), field_color),
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
    draw_keymap_hint(frame, hint_area, "Esc back  j/k move  Enter edit");
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
    draw_modal_hint(frame, input_area, "Esc abort  Enter submit", Color::Blue);
    if input_area.width > 2 && input_area.height > 2 {
        frame.set_cursor_position(Position::new(
            input_area.x + 1 + (prompt.offset as u16).min(input_area.width.saturating_sub(2)),
            input_area.y + 1,
        ));
    }
}

fn draw_error_screen(frame: &mut Frame, error: &ErrorScreen) {
    let main_area = build_title_border(frame);
    let error_area = centered_rect(main_area, main_area.width.min(60), main_area.height.min(8));

    let message = Paragraph::new(error.message.as_str())
        .block(Block::bordered().style(Color::Red).title("Error"))
        .style(Color::Red)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(message, error_area);
    draw_modal_hint(frame, error_area, "Esc/Enter close", Color::Red);
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

fn split_header_content_footer(area: Rect) -> (Rect, Rect, Rect) {
    let (header_area, body_area) = split_header_content(area);
    let hint_height = body_area.height.min(1);
    let content_area = Rect {
        x: body_area.x,
        y: body_area.y,
        width: body_area.width,
        height: body_area.height.saturating_sub(hint_height),
    };
    let hint_area = Rect {
        x: body_area.x,
        y: body_area.y.saturating_add(content_area.height),
        width: body_area.width,
        height: hint_height,
    };

    (header_area, content_area, hint_area)
}

fn draw_keymap_hint(frame: &mut Frame, area: Rect, hint: &str) {
    let hint = Paragraph::new(hint)
        .style(Color::DarkGray)
        .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(hint, area);
}

fn draw_modal_hint(frame: &mut Frame, area: Rect, hint: &str, color: Color) {
    if area.width <= 2 || area.height == 0 {
        return;
    }

    let hint_area = Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(area.height.saturating_sub(1)),
        width: area.width.saturating_sub(2),
        height: 1,
    };
    let hint = Paragraph::new(hint)
        .style(color)
        .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(hint, hint_area);
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

fn build_log_tab_header(frame: &mut Frame, header: Rect, index: usize, follow: bool) {
    let tab = ratatui::widgets::Tabs::new(LOG_TABS)
        .style(Color::White)
        .highlight_style(Color::Blue)
        .select(index)
        .divider(symbols::DOT)
        .padding(" ", " ");
    frame.render_widget(tab, header);

    let status = if follow { "FOLLOW" } else { "PAUSED" };
    let status_width = header.width.min(status.len() as u16);
    let status_area = Rect {
        x: header
            .x
            .saturating_add(header.width.saturating_sub(status_width)),
        y: header.y,
        width: status_width,
        height: header.height,
    };
    let status = Paragraph::new(status)
        .style(if follow { Color::Green } else { Color::Yellow })
        .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(status, status_area);
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
