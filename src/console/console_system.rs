use std::{
    sync::{LazyLock, Mutex},
    time::Duration,
};

use crossterm::event::KeyCode;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{
        Constraint::{Fill, Length},
        Direction::Vertical,
        Layout, Rect,
    },
    style::{Color, Style, Styled},
    symbols,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};
use wasserxr::{Uuid, attacher, detacher, scene::Scene, system};

static TERMINAL: LazyLock<Mutex<Option<DefaultTerminal>>> = LazyLock::new(|| Mutex::new(None));

const TABS: [&str; 4] = ["Entities", "Plugins", "Systems", "Log"];

#[derive(Default, Clone, Copy)]
pub struct AppState {
    pub tab: usize,
    pub entity_selected: usize,
    pub entity_details: bool,
    pub plugin_selected: usize,
    pub system_selected: usize,
}

impl AppState {
    fn tab_next(mut self) -> AppState {
        self.tab = wrap_next(self.tab, TABS.len());
        self
    }

    fn tab_prev(mut self) -> AppState {
        self.tab = wrap_prev(self.tab, TABS.len());
        self
    }

    fn entity_selected_next(mut self, entity_len: usize) -> AppState {
        self.entity_selected = wrap_next(self.entity_selected, entity_len);
        self
    }

    fn entity_selected_prev(mut self, entity_len: usize) -> AppState {
        self.entity_selected = wrap_prev(self.entity_selected, entity_len);
        self
    }

    fn entity_details(mut self, state: bool) -> AppState {
        self.entity_details = state;
        self
    }
}

fn wrap_next(index: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        let index = index % len;
        if index + 1 == len { 0 } else { index + 1 }
    }
}

fn wrap_prev(index: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        let index = index % len;
        if index == 0 { len - 1 } else { index - 1 }
    }
}

#[system(entities=[["Console"]])]
fn console(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    // Get console entity
    if entities[0].is_empty() {
        return;
    }
    let console_entity = entities[0][0];

    // Get current state
    let Ok((state,)) = scene.query::<(&AppState,)>(console_entity, "Console", &["state"]) else {
        return;
    };

    // Handle input
    let state = if let Some(key) = get_input() {
        // Comptue new state
        let new_state = handle_input(scene, *state, key);

        // Set the new state
        let Ok((state,)) =
            scene.query_mut::<(&mut AppState,)>(console_entity, "Console", &["state"])
        else {
            return;
        };
        *state = new_state;
        *state
    } else {
        *state
    };

    if let Ok(mut terminal) = TERMINAL.lock()
        && let Some(terminal) = terminal.as_mut()
    {
        let _ = terminal.draw(|frame| draw(state, scene, frame));
    }
}

fn handle_input(scene: &mut Scene, state: AppState, key: KeyCode) -> AppState {
    match (key, state) {
        // Global Keybinds
        (KeyCode::Char('q'), _) => {
            scene.should_exit();
            state
        }
        (KeyCode::Char('r'), _) => {
            let _ = scene.reload();
            state
        }
        (KeyCode::Right | KeyCode::Char('l'), _) => state.tab_next(),
        (KeyCode::Left | KeyCode::Char('h'), _) => state.tab_prev(),

        // Entity Screen
        (KeyCode::Down | KeyCode::Char('j'), AppState { tab: 0, .. }) => {
            state.entity_selected_next(scene.get_entities().len())
        }
        (KeyCode::Up | KeyCode::Char('k'), AppState { tab: 0, .. }) => {
            state.entity_selected_prev(scene.get_entities().len())
        }
        (
            KeyCode::Enter,
            AppState {
                tab: 0,
                entity_details: false,
                ..
            },
        ) => state.entity_details(true),

        // Entity Details
        (
            KeyCode::Esc,
            AppState {
                tab: 0,
                entity_details: true,
                ..
            },
        ) => state.entity_details(false),
        _ => state,
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

fn draw(state: AppState, scene: &mut Scene, frame: &mut Frame) {
    // Title bar
    let total_area = frame.area();
    let logo_border = Block::bordered()
        .title("WasserXR Console")
        .style(Color::Blue);
    let inner_area = logo_border.inner(total_area);
    frame.render_widget(logo_border, total_area);

    // Header and Main Content split
    let main_layout = Layout::default()
        .direction(Vertical)
        .constraints(vec![Length(1), Fill(1)])
        .split(inner_area);
    let tab_area = main_layout[0];
    let content_area = main_layout[1];

    // Tab Bar
    let tab = ratatui::widgets::Tabs::new(TABS)
        .style(Color::White)
        .highlight_style(Color::Blue)
        .select(state.tab)
        .divider(symbols::DOT)
        .padding(" ", " ");
    frame.render_widget(tab, tab_area);

    match state {
        AppState {
            tab: 0,
            entity_details: false,
            ..
        } => {
            draw_entities_list(state, scene, frame, content_area);
        }
        _ => {
            let error = Paragraph::new("Invalid State")
                .style(Color::Red)
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(error, content_area);
        }
    }
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

fn draw_entities_list(state: AppState, scene: &mut Scene, frame: &mut Frame, area: Rect) {
    let entities: Vec<ListItem> = scene
        .get_entities()
        .iter()
        .map(|id| {
            ListItem::new(left_right_line(
                Span::raw(scene.get_entity_name(*id).unwrap().to_string()),
                Span::styled(format!("{}", *id), Color::DarkGray),
                area.width,
            ))
        })
        .collect();

    let list = List::new(entities)
        .style(Color::White)
        .highlight_style(Color::Blue);

    let mut list_state = ListState::default();
    list_state.select(Some(state.entity_selected));

    frame.render_stateful_widget(list, area, &mut list_state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previous_tab_wraps_from_first_to_last() {
        let state = AppState::default().tab_prev();

        assert_eq!(state.tab, TABS.len() - 1);
    }

    #[test]
    fn next_tab_wraps_from_last_to_first() {
        let state = AppState {
            tab: TABS.len() - 1,
            ..Default::default()
        }
        .tab_next();

        assert_eq!(state.tab, 0);
    }

    #[test]
    fn previous_entity_wraps_from_first_to_last() {
        let state = AppState::default().entity_selected_prev(3);

        assert_eq!(state.entity_selected, 2);
    }

    #[test]
    fn entity_selection_ignores_empty_lists() {
        let state = AppState {
            entity_selected: 4,
            ..Default::default()
        };

        assert_eq!(state.entity_selected_next(0).entity_selected, 0);
        assert_eq!(state.entity_selected_prev(0).entity_selected, 0);
    }

    #[test]
    fn entity_selection_normalizes_out_of_range_indexes() {
        let state = AppState {
            entity_selected: usize::MAX,
            ..Default::default()
        };

        assert_eq!(state.entity_selected_next(3).entity_selected, 1);
        assert_eq!(state.entity_selected_prev(3).entity_selected, 2);
    }
}
