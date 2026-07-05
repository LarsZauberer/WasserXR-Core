use std::time::Duration;

use glium::winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::EventLoop,
    platform::pump_events::EventLoopExtPumpEvents,
};
use wasserxr::{Uuid, scene::Scene, system};

struct InputGetterApp<'a> {
    events: &'a mut Vec<WindowEvent>,
}

impl<'a> ApplicationHandler for InputGetterApp<'a> {
    fn resumed(&mut self, _event_loop: &glium::winit::event_loop::ActiveEventLoop) {
        // Empty function that doesn't need to to anything
    }

    fn window_event(
        &mut self,
        _event_loop: &glium::winit::event_loop::ActiveEventLoop,
        _window_id: glium::winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.events.push(event);
    }
}

pub(crate) fn get_event_loop(scene: &mut Scene) -> &mut EventLoop<()> {
    if scene
        .get_resource::<EventLoop<()>>("window_event_loop")
        .is_err()
    {
        let _ = scene.add_resource::<EventLoop<()>>(
            "window_event_loop".to_owned(),
            glium::winit::event_loop::EventLoop::new().expect("Failed to create EventLoop"),
        );
    }

    scene
        .get_mut_resource::<EventLoop<()>>("window_event_loop")
        .expect("Failed to get the EventLoop")
}

#[system]
fn window_input_read(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    let mut events: Vec<WindowEvent> = Vec::new();
    let mut app = InputGetterApp {
        events: &mut events,
    };
    get_event_loop(scene).pump_app_events(Some(Duration::ZERO), &mut app);

    if let Ok(scene_events) = scene.get_mut_resource::<Vec<WindowEvent>>("window_events") {
        *scene_events = events;
    } else {
        let _ = scene.add_resource::<Vec<WindowEvent>>("window_events".to_owned(), Vec::new());
    }
}

#[system]
fn window_input_reset(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    let Ok(events) = scene.get_mut_resource::<Vec<WindowEvent>>("window_events") else {
        let _ = scene.add_resource::<Vec<WindowEvent>>("window_events".to_owned(), Vec::new());
        return;
    };
    events.clear();
}

#[system]
fn window_close(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    let Ok(events) = scene.get_mut_resource::<Vec<WindowEvent>>("window_events") else {
        let _ = scene.add_resource::<Vec<WindowEvent>>("window_events".to_owned(), Vec::new());
        return;
    };

    if events.contains(&WindowEvent::CloseRequested) {
        scene.should_exit();
    }
}

#[system]
fn make_window(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    let camera = scene.add_entity();
    let _ = scene.set_entity_name(camera, "Camera".to_owned());
    let _ = scene.add_component(camera, "Camera".to_owned());
    let _ = scene.add_component(camera, "Transform".to_owned());

    let _ = scene.add_system("window_input_reset".to_owned(), 1);
    let _ = scene.add_system("window_input_read".to_owned(), 1000);
    let _ = scene.add_system("renderer".to_owned(), 100);
    let _ = scene.add_system("window_close".to_owned(), 200);

    let _ = scene.remove_system("make_window");
}
