use ratatui::DefaultTerminal;
use wasserxr::component;

use crate::console::console_app::ConsoleApp;

#[component]
#[derive(Default)]
struct Console {
    #[getter]
    #[mutable]
    terminal: Option<DefaultTerminal>,

    #[getter]
    #[mutable]
    app: Option<ConsoleApp>,
}
