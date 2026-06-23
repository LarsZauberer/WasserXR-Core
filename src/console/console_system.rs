use ratatui::DefaultTerminal;
use wasserxr::{Uuid, scene::Scene, system};

use crate::{
    console::{
        console_app::{Action, ConsoleApp},
        console_data::ConsoleData,
    },
    errors::CoreError,
};

#[system(entities=[["Console"]])]
fn console(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    // Get console entity
    if entities[0].is_empty() {
        return;
    }
    let console_entity = entities[0][0];

    // Initialize
    if init_console(scene, console_entity).is_err() {
        return;
    };

    if update_console_data(scene, console_entity).is_err() {
        return;
    }

    let action = {
        let Ok((Some(terminal), Some(app))) =
            scene.query_mut::<(&mut Option<DefaultTerminal>, &mut Option<ConsoleApp>)>(
                console_entity,
                "Console",
                &["terminal", "app"],
            )
        else {
            return;
        };

        let _ = terminal.draw(|frame| app.draw(frame));

        app.get_action()
    };

    match action {
        Action::Quit => {
            ratatui::restore();
            scene.should_exit();
        }
        Action::Reload => {
            let _ = scene.reload();
        }
        _ => {}
    }
}

fn init_console(scene: &mut Scene, console_entity: Uuid) -> Result<(), CoreError> {
    // Check if the console has been initialized yet otherwise initialize it
    let (terminal, app): (&mut Option<DefaultTerminal>, &mut Option<ConsoleApp>) = scene
        .query_mut(console_entity, "Console", &["terminal", "app"])
        .map_err(|_| CoreError::FieldNotFound)?;

    if terminal.is_none() {
        *terminal = Some(ratatui::init());
    }

    if app.is_none() {
        *app = Some(ConsoleApp::default());
    }

    Ok(())
}

fn update_console_data(scene: &mut Scene, console_entity: Uuid) -> Result<(), CoreError> {
    let entities: Vec<(Uuid, String)> = scene
        .get_entities()
        .iter()
        .map(|id| (*id, scene.get_entity_name(*id).unwrap().to_owned()))
        .collect();

    let data = ConsoleData::new(entities);

    let (Some(app),): (&mut Option<ConsoleApp>,) = scene
        .query_mut(console_entity, "Console", &["app"])
        .map_err(|_| CoreError::FieldNotFound)?
    else {
        return Err(CoreError::NotInitialized);
    };

    app.set_data(data);

    Ok(())
}
