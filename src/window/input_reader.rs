use std::time::Duration;

use glium::{
    Surface,
    winit::{
        application::ApplicationHandler, event::WindowEvent, event_loop::EventLoop,
        platform::pump_events::EventLoopExtPumpEvents,
    },
};
use wasserxr::{Uuid, debug, scene::Scene, system, warn};

use crate::window::window::Display;

struct InputGetterApp<'a> {
    events: &'a mut Vec<WindowEvent>,
}

impl<'a> ApplicationHandler for InputGetterApp<'a> {
    fn resumed(&mut self, _event_loop: &glium::winit::event_loop::ActiveEventLoop) {
        // Empty function that doesn't need to to anything
    }

    fn window_event(
        &mut self,
        event_loop: &glium::winit::event_loop::ActiveEventLoop,
        window_id: glium::winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.events.push(event);
    }
}

#[system(entities=[["Window"]])]
fn window_input_read(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    for window_entity in &entities[0] {
        let window_entity = *window_entity;

        let (event_loop, events) = match scene
            .query_mut::<(&mut EventLoop<()>, &mut Vec<WindowEvent>)>(
                window_entity,
                "Window",
                &["event_loop", "events"],
            ) {
            Ok(res) => res,
            Err(err) => {
                warn!(scene, "No events on Entity `{}`: {:?}", window_entity, err);
                continue;
            }
        };

        let mut app = InputGetterApp { events };
        event_loop.pump_app_events(Some(Duration::ZERO), &mut app);
    }
}

#[system(entities=[["Window"]])]
fn window_input_reset(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    for window_entity in &entities[0] {
        let window_entity = *window_entity;

        let (event_list,) =
            match scene.query_mut::<(&mut Vec<WindowEvent>,)>(window_entity, "Window", &["events"])
            {
                Ok(res) => res,
                Err(err) => {
                    warn!(scene, "No events on Entity `{}`: {:?}", window_entity, err);
                    continue;
                }
            };

        event_list.clear();
    }
}

#[system(entities=[["Window"]])]
fn window_clear(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    for window_entity in &entities[0] {
        let window_entity = *window_entity;

        let (display,) =
            match scene.query_mut::<(&mut Display,)>(window_entity, "Window", &["display"]) {
                Ok(res) => res,
                Err(err) => {
                    warn!(scene, "No display on Entity `{}`: {:?}", window_entity, err);
                    continue;
                }
            };

        let mut frame = display.draw();
        frame.clear_color(0.01, 0.01, 0.01, 1.0);
        frame.finish().unwrap();
    }
}

#[system(entities=[["Window"]])]
fn window_log_events(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    for window_entity in &entities[0] {
        let window_entity = *window_entity;

        let (event_list,) =
            match scene.query::<(&Vec<WindowEvent>,)>(window_entity, "Window", &["events"]) {
                Ok(res) => res,
                Err(err) => {
                    warn!(scene, "No events on Entity `{}`: {:?}", window_entity, err);
                    continue;
                }
            };

        for i in event_list {
            debug!(scene, "{:?}", i)
        }
    }
}

#[system(entities=[["Window"]])]
fn window_close(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    for window_entity in &entities[0] {
        let window_entity = *window_entity;

        let (event_list,) =
            match scene.query::<(&Vec<WindowEvent>,)>(window_entity, "Window", &["events"]) {
                Ok(res) => res,
                Err(err) => {
                    warn!(scene, "No events on Entity `{}`: {:?}", window_entity, err);
                    continue;
                }
            };

        if event_list.contains(&WindowEvent::CloseRequested) {
            let _ = scene.remove_component(window_entity, "Window");
        }
    }
}

#[system]
fn make_window(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    let window = scene.add_entity();
    let _ = scene.set_entity_name(window, "Window".to_owned());

    let _ = scene.add_component(window, "Window".to_owned());

    let _ = scene.add_system("window_input_reset".to_owned(), 1);
    let _ = scene.add_system("window_input_read".to_owned(), 1000);
    let _ = scene.add_system("window_clear".to_owned(), 100);
    let _ = scene.add_system("window_close".to_owned(), 200);

    let _ = scene.remove_system("make_window");
}
