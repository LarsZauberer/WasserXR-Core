use std::{
    io::{self, Stdout},
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use wasserxr::{Uuid, attacher, detacher, scene::Scene, system};

static TICKS: AtomicU64 = AtomicU64::new(0);
static CONSOLE_WORKER: LazyLock<Mutex<Option<ConsoleWorker>>> = LazyLock::new(|| Mutex::new(None));

enum UiCommand {
    Tick(u64),
    Shutdown,
}

struct ConsoleWorker {
    command_tx: Sender<UiCommand>,
    handle: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct UiState {
    tick: u64,
    input: String,
    submitted: String,
    last_key: String,
}

#[system]
pub fn console(_scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    // ponytail: the system only publishes data; the UI loop lives in the worker.
    let tick = TICKS.fetch_add(1, Ordering::Relaxed).saturating_add(1);
    send_tick(tick);
}

#[attacher(console)]
pub fn attach_console(_scene: &mut Scene) {
    start_console_worker();
}

#[detacher(console)]
pub fn detach_console(_scene: &mut Scene) {
    stop_console_worker();
}

fn start_console_worker() {
    let Ok(mut worker) = CONSOLE_WORKER.lock() else {
        return;
    };

    if worker
        .as_ref()
        .and_then(|worker| worker.handle.as_ref())
        .is_some_and(|handle| !handle.is_finished())
    {
        return;
    }

    if let Some(mut old_worker) = worker.take()
        && let Some(handle) = old_worker.handle.take()
    {
        let _ = handle.join();
    }

    let (command_tx, command_rx) = mpsc::channel();
    let handle = thread::spawn(move || run_console_worker(command_rx));

    *worker = Some(ConsoleWorker {
        command_tx,
        handle: Some(handle),
    });
}

fn stop_console_worker() {
    let worker = CONSOLE_WORKER
        .lock()
        .ok()
        .and_then(|mut worker| worker.take());

    if let Some(mut worker) = worker {
        let _ = worker.command_tx.send(UiCommand::Shutdown);

        if let Some(handle) = worker.handle.take() {
            let _ = handle.join();
        }
    }
}

fn send_tick(tick: u64) {
    let Ok(mut worker_slot) = CONSOLE_WORKER.lock() else {
        return;
    };

    let Some(worker) = worker_slot.as_mut() else {
        return;
    };

    if worker.command_tx.send(UiCommand::Tick(tick)).is_err() {
        *worker_slot = None;
    }
}

fn run_console_worker(command_rx: Receiver<UiCommand>) {
    let mut state = UiState::default();
    let Ok(mut terminal) = enter_terminal() else {
        return;
    };

    loop {
        for command in command_rx.try_iter() {
            match command {
                UiCommand::Tick(tick) => state.tick = tick,
                UiCommand::Shutdown => {
                    restore_terminal(terminal);
                    return;
                }
            }
        }

        poll_input(&mut state);

        let _ = terminal.draw(|frame| render_console(frame, &state));
        thread::sleep(Duration::from_millis(16));
    }
}

type ConsoleTerminal = Terminal<CrosstermBackend<Stdout>>;

fn enter_terminal() -> io::Result<ConsoleTerminal> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;

    if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide) {
        let _ = disable_raw_mode();
        return Err(error);
    }

    match Terminal::new(CrosstermBackend::new(stdout)) {
        Ok(terminal) => Ok(terminal),
        Err(error) => {
            let _ = disable_raw_mode();
            Err(error)
        }
    }
}

fn restore_terminal(mut terminal: ConsoleTerminal) {
    let _ = execute!(terminal.backend_mut(), Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = terminal.show_cursor();
}

fn poll_input(state: &mut UiState) {
    while matches!(event::poll(Duration::from_millis(1)), Ok(true)) {
        let Ok(Event::Key(key)) = event::read() else {
            continue;
        };

        handle_key(key, state);
    }
}

fn handle_key(key: KeyEvent, state: &mut UiState) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    state.last_key = describe_key(key);

    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.submitted = "close requested".to_owned();
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            state.submitted = "close requested".to_owned();
        }
        KeyCode::Char(character) => {
            state.input.push(character);
        }
        KeyCode::Backspace => {
            state.input.pop();
        }
        KeyCode::Enter => {
            state.submitted = state.input.clone();
            state.input.clear();
        }
        _ => {}
    }
}

fn describe_key(key: KeyEvent) -> String {
    match key.code {
        KeyCode::Char(character) if key.modifiers.is_empty() => character.to_string(),
        KeyCode::Char(character) => format!("{:?}+{}", key.modifiers, character),
        key_code => format!("{key_code:?}"),
    }
}

fn render_console(frame: &mut Frame<'_>, state: &UiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(frame.area());

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Tick: ", Style::default().fg(Color::Cyan)),
            Span::raw(state.tick.to_string()),
            Span::raw("  "),
            Span::styled("Last key: ", Style::default().fg(Color::Cyan)),
            Span::raw(state.last_key.as_str()),
        ]))
        .block(
            Block::default()
                .title("WasserXR Console")
                .borders(Borders::ALL),
        ),
        chunks[0],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Input: ", Style::default().fg(Color::Green)),
            Span::raw(state.input.as_str()),
        ]))
        .block(Block::default().borders(Borders::ALL)),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Submitted: ", Style::default().fg(Color::Yellow)),
            Span::raw(state.submitted.as_str()),
        ]))
        .block(Block::default().borders(Borders::ALL)),
        chunks[2],
    );

    frame.render_widget(
        Paragraph::new("Type text, Backspace edits, Enter submits, q/Esc requests close.")
            .block(Block::default().borders(Borders::ALL)),
        chunks[3],
    );
}
