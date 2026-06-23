use ratatui::DefaultTerminal;
use wasserxr::component;

use crate::console::console_system::AppState;

#[component]
#[derive(Default)]
struct Console {
    #[mutable]
    state: AppState,
}
