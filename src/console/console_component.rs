use wasserxr::component;

use crate::console::state::AppState;

#[component]
#[derive(Default)]
struct Console {
    #[mutable]
    state: AppState,
}
