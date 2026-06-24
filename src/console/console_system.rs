use std::{
    sync::{LazyLock, Mutex},
    time::Duration,
};

use crossterm::event::KeyCode;
use ratatui::DefaultTerminal;
use wasserxr::{Uuid, attacher, detacher, scene::Scene, system};

use crate::console::state::AppState;

static TERMINAL: LazyLock<Mutex<Option<DefaultTerminal>>> = LazyLock::new(|| Mutex::new(None));

#[system(entities=[["Console"]])]
fn console(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    // Get console entity
    if entities[0].is_empty() {
        return;
    }
    let console_entity = entities[0][0];

    let Ok((state,)) = scene.query::<(&AppState,)>(console_entity, "Console", &["state"]) else {
        return;
    };
    let state = *state;

    let state = if let Some(key) = get_input() {
        let new_state = handle_input(scene, state, key);

        let Ok((state,)) =
            scene.query_mut::<(&mut AppState,)>(console_entity, "Console", &["state"])
        else {
            return;
        };
        *state = new_state;
        *state
    } else {
        state
    };

    if let Ok(mut terminal) = TERMINAL.lock()
        && let Some(terminal) = terminal.as_mut()
    {
        let _ = terminal.draw(|frame| state.draw(scene, frame, frame.area()));
    }
}

fn handle_input(scene: &mut Scene, state: AppState, key: KeyCode) -> AppState {
    match key {
        KeyCode::Char('q') => {
            scene.should_exit();
            state
        }
        KeyCode::Char('r') => {
            let _ = scene.reload();
            state
        }
        _ => state
            .handle_input(key)
            .normalize_entity_selection(scene.get_entities().len()),
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
