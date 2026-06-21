use std::{sync::Mutex, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use log::error;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use wasserxr::{Uuid, attacher, component, detacher, scene::Scene, system};

static TERMINAL: Mutex<Option<DefaultTerminal>> = Mutex::new(None);
static SELECTED_ENTITY: Mutex<Option<usize>> = Mutex::new(None);

#[system]
pub fn console(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    let mut entities = scene.get_entities();

    if event::poll(Duration::from_millis(0)).expect("Crossterm event polling failed") {
        match event::read().expect("Crossterm event grabbing failed") {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if handle_key(key.code, scene, entities.len()) {
                    entities = scene.get_entities();
                }
            }
            _ => {}
        }
    }

    let selected = normalize_selection(entities.len());

    let mut terminal = TERMINAL.lock().expect("Failed to lock terminal");
    let Some(terminal) = terminal.as_mut() else {
        error!("The Terminal hasn't been initialized");
        return;
    };

    terminal
        .draw(|frame| render(frame, &entities, selected))
        .expect("Draw call failed");
}

#[attacher(console)]
pub fn console_attacher(_scene: &mut Scene) {
    let mut terminal = TERMINAL.lock().expect("Failed to lock terminal");

    if terminal.is_none() {
        *terminal = Some(ratatui::init());
    }
}

#[detacher(console)]
pub fn console_detacher(_scene: &mut Scene) {
    *TERMINAL.lock().expect("Failed to lock terminal") = None;
    *SELECTED_ENTITY.lock().expect("Failed to lock selection") = None;
    ratatui::restore();
}

fn handle_key(key: KeyCode, scene: &mut Scene, entity_count: usize) -> bool {
    match key {
        KeyCode::Char('q') => {
            scene.should_exit();
            true
        }
        KeyCode::Char('e') => {
            scene.add_entity();
            true
        }
        KeyCode::Up | KeyCode::Char('k') => {
            select_previous(entity_count);
            false
        }
        KeyCode::Down | KeyCode::Char('j') => {
            select_next(entity_count);
            false
        }
        _ => false,
    }
}

fn select_previous(entity_count: usize) {
    let mut selected = SELECTED_ENTITY.lock().expect("Failed to lock selection");

    *selected = match (entity_count, *selected) {
        (0, _) => None,
        (_, None | Some(0)) => Some(0),
        (_, Some(index)) => Some(index - 1),
    };
}

fn select_next(entity_count: usize) {
    let mut selected = SELECTED_ENTITY.lock().expect("Failed to lock selection");

    *selected = match (entity_count, *selected) {
        (0, _) => None,
        (_, None) => Some(0),
        (count, Some(index)) => Some(index.saturating_add(1).min(count - 1)),
    };
}

fn normalize_selection(entity_count: usize) -> Option<usize> {
    let mut selected = SELECTED_ENTITY.lock().expect("Failed to lock selection");

    *selected = match (entity_count, *selected) {
        (0, _) => None,
        (count, Some(index)) => Some(index.min(count - 1)),
        (_, None) => Some(0),
    };

    *selected
}

fn render(frame: &mut Frame, entities: &[Uuid], selected: Option<usize>) {
    let area = frame.area();

    let root = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(5),
        ])
        .split(area);

    render_header(frame, root[0], entities.len(), selected);
    render_main(frame, root[1], entities, selected);
    render_help(frame, root[2]);
}

fn render_header(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    entity_count: usize,
    selected: Option<usize>,
) {
    let selected_text = selected
        .map(|index| format!("selected {}", index + 1))
        .unwrap_or_else(|| "nothing selected".to_owned());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "WasserXR Console",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{entity_count} entities"),
            Style::default().fg(Color::LightGreen),
        ),
        Span::raw("  "),
        Span::styled(selected_text, Style::default().fg(Color::Yellow)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(header, area);
}

fn render_main(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    entities: &[Uuid],
    selected: Option<usize>,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(area);

    render_entity_list(frame, chunks[0], entities, selected);
    render_details(frame, chunks[1], entities, selected);
}

fn render_entity_list(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    entities: &[Uuid],
    selected: Option<usize>,
) {
    let items = if entities.is_empty() {
        vec![ListItem::new(Line::styled(
            "No entities yet. Press e to add one.",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        entities
            .iter()
            .enumerate()
            .map(|(index, entity)| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:>3}", index + 1),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw("  "),
                    Span::styled(entity.to_string(), Style::default().fg(Color::White)),
                ]))
            })
            .collect()
    };

    let mut state = ListState::default().with_selected(selected);
    let list = List::new(items)
        .block(
            Block::default()
                .title(" Entities ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::LightBlue)),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_details(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    entities: &[Uuid],
    selected: Option<usize>,
) {
    let selected_entity = selected.and_then(|index| entities.get(index));
    let detail_lines = match selected_entity {
        Some(entity) => vec![
            Line::from(Span::styled(
                "Selected Entity",
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::styled("UUID: ", Style::default().fg(Color::Yellow)),
                Span::styled(entity.to_string(), Style::default().fg(Color::White)),
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                "Use Up/Down or k/j to move.",
                Style::default().fg(Color::Gray),
            )),
        ],
        None => vec![
            Line::from(Span::styled(
                "No entity selected",
                Style::default().fg(Color::DarkGray),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "Press e to create the first entity.",
                Style::default().fg(Color::Gray),
            )),
        ],
    };

    let details = Paragraph::new(detail_lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );

    frame.render_widget(details, area);
}

fn render_help(frame: &mut Frame, area: ratatui::layout::Rect) {
    let help = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("e", Style::default().fg(Color::LightGreen)),
            Span::raw(" add entity    "),
            Span::styled("Up/k", Style::default().fg(Color::LightCyan)),
            Span::raw(" previous    "),
            Span::styled("Down/j", Style::default().fg(Color::LightCyan)),
            Span::raw(" next"),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "Entities appear in the embedded list as soon as the scene reports them.",
            Style::default().fg(Color::Gray),
        )),
    ])
    .block(
        Block::default()
            .title(" Controls ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );

    frame.render_widget(help, area);
}
