use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Rect,
    style::Color,
    symbols,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};
use wasserxr::scene::Scene;

const TABS: [&str; 4] = ["Entities", "Plugins", "Systems", "Log"];

#[derive(Default, Clone, Copy)]
pub struct AppState {
    pub tab: usize,
    pub entity_list: EntityListState,
    pub entity_detail: EntityDetailState,
    pub plugin_list: PluginListState,
    pub system_list: SystemListState,
}

impl AppState {
    pub fn handle_input(mut self, key: KeyCode) -> AppState {
        match key {
            KeyCode::Right | KeyCode::Char('l') => self.tab_next(),
            KeyCode::Left | KeyCode::Char('h') => self.tab_prev(),
            _ if self.tab == 0 && self.entity_detail.active => {
                self.entity_detail = self.entity_detail.handle_input(key);
                self
            }
            _ if self.tab == 0 => {
                if key == KeyCode::Enter {
                    self.entity_detail = EntityDetailState {
                        active: true,
                        selected: self.entity_list.selected,
                    };
                } else {
                    self.entity_list = self.entity_list.handle_input(key);
                }
                self
            }
            _ if self.tab == 1 => {
                self.plugin_list = self.plugin_list.handle_input(key);
                self
            }
            _ if self.tab == 2 => {
                self.system_list = self.system_list.handle_input(key);
                self
            }
            _ => self,
        }
    }

    pub fn draw(&self, scene: &mut Scene, frame: &mut Frame, area: Rect) {
        // WARN: It is not possible to call `Layout` because it introduces a new thread, that will
        // destroy Hotreloading.
        let logo_border = Block::bordered()
            .title("WasserXR Console")
            .style(Color::Blue);
        let inner_area = logo_border.inner(area);
        frame.render_widget(logo_border, area);

        let tab_area = Rect {
            x: inner_area.x,
            y: inner_area.y,
            width: inner_area.width,
            height: inner_area.height.min(1),
        };

        let content_area = Rect {
            x: inner_area.x,
            y: inner_area.y.saturating_add(tab_area.height),
            width: inner_area.width,
            height: inner_area.height.saturating_sub(tab_area.height),
        };

        let tab = ratatui::widgets::Tabs::new(TABS)
            .style(Color::White)
            .highlight_style(Color::Blue)
            .select(self.tab)
            .divider(symbols::DOT)
            .padding(" ", " ");
        frame.render_widget(tab, tab_area);

        match self.tab {
            0 if self.entity_detail.active => self.entity_detail.draw(scene, frame, content_area),
            0 => self.entity_list.draw(scene, frame, content_area),
            1 => self.plugin_list.draw(scene, frame, content_area),
            2 => self.system_list.draw(scene, frame, content_area),
            _ => draw_placeholder(frame, content_area, "Log"),
        }
    }

    pub fn normalize_entity_selection(mut self, entity_len: usize) -> AppState {
        self.entity_list = self.entity_list.normalize(entity_len);
        if !self.entity_detail.active {
            self.entity_detail.selected = self.entity_list.selected;
        }
        self
    }

    fn tab_next(mut self) -> AppState {
        self.tab = wrap_next(self.tab, TABS.len());
        self
    }

    fn tab_prev(mut self) -> AppState {
        self.tab = wrap_prev(self.tab, TABS.len());
        self
    }
}

#[derive(Default, Clone, Copy)]
pub struct EntityListState {
    pub selected: usize,
}

impl EntityListState {
    pub fn handle_input(mut self, key: KeyCode) -> EntityListState {
        match key {
            KeyCode::Down | KeyCode::Char('j') => self.selected = self.selected.wrapping_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.wrapping_sub(1),
            _ => {}
        }
        self
    }

    pub fn draw(&self, scene: &mut Scene, frame: &mut Frame, area: Rect) {
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
                    area.width,
                ))
            })
            .collect();

        let list = List::new(entities)
            .style(Color::White)
            .highlight_style(Color::Blue);

        let mut list_state = ListState::default();
        if !entity_ids.is_empty() {
            list_state.select(Some(normalize_index(self.selected, entity_ids.len())));
        }

        frame.render_stateful_widget(list, area, &mut list_state);
    }

    fn normalize(mut self, len: usize) -> EntityListState {
        self.selected = normalize_index(self.selected, len);
        self
    }
}

#[derive(Default, Clone, Copy)]
pub struct EntityDetailState {
    pub active: bool,
    pub selected: usize,
}

impl EntityDetailState {
    pub fn handle_input(mut self, key: KeyCode) -> EntityDetailState {
        if key == KeyCode::Esc {
            self.active = false;
        }
        self
    }

    pub fn draw(&self, scene: &mut Scene, frame: &mut Frame, area: Rect) {
        let entity_ids = scene.get_entities();
        let Some(id) = entity_ids.get(normalize_index(self.selected, entity_ids.len())) else {
            draw_placeholder(frame, area, "No entity selected");
            return;
        };

        let name = scene
            .get_entity_name(*id)
            .map(ToString::to_string)
            .unwrap_or_else(|_| "Unnamed".to_string());

        let details = Paragraph::new(vec![
            Line::from(Span::styled(name, Color::Blue)),
            Line::from(Span::raw(format!("Entity: {}", *id))),
            Line::from(Span::styled("Press Esc to return", Color::DarkGray)),
        ])
        .style(Color::White);

        frame.render_widget(details, area);
    }
}

#[derive(Default, Clone, Copy)]
pub struct PluginListState {
    pub selected: usize,
}

impl PluginListState {
    pub fn handle_input(mut self, key: KeyCode) -> PluginListState {
        match key {
            KeyCode::Down | KeyCode::Char('j') => self.selected = self.selected.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            _ => {}
        }
        self
    }

    pub fn draw(&self, _scene: &mut Scene, frame: &mut Frame, area: Rect) {
        draw_placeholder(frame, area, "Plugins");
    }
}

#[derive(Default, Clone, Copy)]
pub struct SystemListState {
    pub selected: usize,
}

impl SystemListState {
    pub fn handle_input(mut self, key: KeyCode) -> SystemListState {
        match key {
            KeyCode::Down | KeyCode::Char('j') => self.selected = self.selected.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            _ => {}
        }
        self
    }

    pub fn draw(&self, _scene: &mut Scene, frame: &mut Frame, area: Rect) {
        draw_placeholder(frame, area, "Systems");
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

fn normalize_index(index: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else if index == usize::MAX {
        len - 1
    } else {
        index % len
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

fn draw_placeholder(frame: &mut Frame, area: Rect, title: &str) {
    let paragraph = Paragraph::new(title)
        .style(Color::DarkGray)
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(paragraph, area);
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
        let state = AppState::default()
            .handle_input(KeyCode::Up)
            .normalize_entity_selection(3);

        assert_eq!(state.entity_list.selected, 2);
    }

    #[test]
    fn entity_selection_ignores_empty_lists() {
        let state = AppState {
            entity_list: EntityListState { selected: 4 },
            ..Default::default()
        };

        assert_eq!(state.normalize_entity_selection(0).entity_list.selected, 0);
    }

    #[test]
    fn entity_selection_normalizes_out_of_range_indexes() {
        let state = AppState {
            entity_list: EntityListState {
                selected: usize::MAX,
            },
            ..Default::default()
        };

        assert_eq!(state.normalize_entity_selection(3).entity_list.selected, 2);
    }

    #[test]
    fn enter_opens_entity_details() {
        let state = AppState {
            entity_list: EntityListState { selected: 2 },
            ..Default::default()
        }
        .handle_input(KeyCode::Enter);

        assert!(state.entity_detail.active);
        assert_eq!(state.entity_detail.selected, 2);
    }

    #[test]
    fn escape_closes_entity_details() {
        let state = AppState {
            entity_detail: EntityDetailState {
                active: true,
                selected: 0,
            },
            ..Default::default()
        }
        .handle_input(KeyCode::Esc);

        assert!(!state.entity_detail.active);
    }
}
