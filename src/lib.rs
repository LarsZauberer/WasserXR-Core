use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
        mpmc::channel,
    },
    thread,
};

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};
use wasserxr::{Uuid, attacher, detacher, scene::Scene, system};

pub mod console;

static SHOULD_EXIT: AtomicBool = AtomicBool::new(true);
static INFO: Mutex<MyConsoleInformation> = Mutex::new(MyConsoleInformation {
    entities: Vec::new(),
});
static ACTION: Mutex<Option<MyConsoleAction>> = Mutex::new(None);

struct MyConsoleInformation {
    pub entities: Vec<Uuid>,
}

enum MyConsoleAction {
    AddEntity,
}

#[system]
pub fn my_console(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    match take_action() {
        Some(MyConsoleAction::AddEntity) => {
            scene.add_entity();
        }
        None => {}
    }

    publish_scene_info(scene);
}

#[attacher(my_console)]
pub fn my_console_attacher(_scene: &mut Scene) {
    SHOULD_EXIT.store(false, Ordering::SeqCst);
    thread::spawn(|| {
        let mut terminal = ratatui::init();
        let result = app(&mut terminal);
        ratatui::restore();
        result.expect("Failure running app");
    });
}

#[detacher(my_console)]
pub fn my_console_detacher(_scene: &mut Scene) {
    SHOULD_EXIT.store(true, Ordering::SeqCst);
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    loop {
        if SHOULD_EXIT.load(Ordering::SeqCst) {
            break;
        }
        terminal.draw(render)?;
        match crossterm::event::read()? {
            Event::Key(key)
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') =>
            {
                break;
            }
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if key.code == KeyCode::Char('e') {
                    set_action(MyConsoleAction::AddEntity);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn set_action(action: MyConsoleAction) {
    *ACTION.lock().expect("Failed to lock action") = Some(action);
}

fn take_action() -> Option<MyConsoleAction> {
    ACTION.lock().expect("Failed to lock action").take()
}

fn publish_scene_info(scene: &Scene) {
    INFO.lock().expect("Failed to lock info").entities = scene.get_entities();
}

fn render(frame: &mut Frame) {
    let info = INFO.lock().expect("Failed to lock info");
    let mut uuids = "".to_owned();
    for i in &info.entities {
        uuids = uuids + &format!("Entity: {}\n", i);
    }
    drop(info);
    frame.render_widget(format!("Hello WasserXR\n{}", uuids), frame.area());
}
