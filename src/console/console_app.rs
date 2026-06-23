use std::time::Duration;

use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Styled},
    symbols,
    widgets::{Block, Paragraph, Tabs},
};

use crate::console::console_data::ConsoleData;

#[derive(Copy, Clone, Default)]
enum Tab {
    #[default]
    Entities,
    Plugins,
    Systems,
    Logs,
}

impl Tab {
    pub fn next(&mut self) {
        match self {
            Self::Entities => {
                *self = Self::Plugins;
            }
            Self::Plugins => {
                *self = Self::Systems;
            }
            Self::Systems => {
                *self = Self::Logs;
            }
            Self::Logs => {
                *self = Self::Logs;
            }
        }
    }

    pub fn prev(&mut self) {
        match self {
            Self::Entities => {
                *self = Self::Entities;
            }
            Self::Plugins => {
                *self = Self::Entities;
            }
            Self::Systems => {
                *self = Self::Plugins;
            }
            Self::Logs => {
                *self = Self::Systems;
            }
        }
    }

    pub fn get_tabs() -> Vec<String> {
        vec![
            "Entities".to_owned(),
            "Plugins".to_owned(),
            "Systems".to_owned(),
            "Logs".to_owned(),
        ]
    }
}

impl From<usize> for Tab {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Entities,
            1 => Self::Plugins,
            2 => Self::Systems,
            3 => Self::Logs,
            _ => Self::default(),
        }
    }
}

impl From<Tab> for usize {
    fn from(val: Tab) -> Self {
        match val {
            Tab::Entities => 0,
            Tab::Plugins => 1,
            Tab::Systems => 2,
            Tab::Logs => 3,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum Action {
    #[default]
    None,
    Quit,
    Reload,
}

#[derive(Default)]
pub struct ConsoleApp {
    console_data: ConsoleData,
    tab_state: Tab,
    action: Action,
}

impl ConsoleApp {
    pub(crate) fn set_data(&mut self, data: ConsoleData) {
        self.console_data = data;
    }

    pub(crate) fn get_action(&mut self) -> Action {
        let res = self.action;
        self.action = Action::None;
        res
    }

    pub(crate) fn draw(&mut self, frame: &mut Frame) {
        self.handle_input();

        let area = frame.area();

        let title_block = Block::bordered()
            .title("WasserXR Console")
            .style(Color::Blue);
        let inner_area = title_block.inner(area);

        let total_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(inner_area);

        let selected: usize = self.tab_state.into();
        let tabs = Tabs::new(Tab::get_tabs())
            .style(Color::White)
            .highlight_style(Color::Blue)
            .select(selected)
            .divider(symbols::DOT)
            .padding(" ", " ");

        let content_block = Block::bordered().style(Color::White);
        let content_area = content_block.inner(total_layout[1]);

        frame.render_widget(title_block, area);
        frame.render_widget(tabs, total_layout[0]);
        frame.render_widget(content_block, total_layout[1]);

        match self.tab_state {
            Tab::Entities => {
                self.draw_entities(frame, content_area);
            }
            _ => {
                let paragraph = Paragraph::new("Not implemented yet!")
                    .style(Color::Red)
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(paragraph, content_area);
            }
        }
    }

    fn handle_input(&mut self) {
        // Make the Key event non blocking
        let Ok(true) = event::poll(Duration::from_secs(0)) else {
            return;
        };

        // Check if we can read events and is a key event
        let Ok(event::Event::Key(key)) = event::read() else {
            return;
        };
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Manage Press Events
        match key.code {
            KeyCode::Char('q') => {
                self.action = Action::Quit;
            }
            KeyCode::Char('r') => {
                self.action = Action::Reload;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.tab_state.next();
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.tab_state.prev();
            }
            _ => {}
        }
    }

    fn draw_entities(&mut self, frame: &mut Frame, area: Rect) {}
}
