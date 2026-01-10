use crate::state_handler::{main_page::MainPageState, setup_sequence::SetupState};

/// State holds the state of the application
#[derive(Default, Debug, Clone)]
pub struct State {
    pub active_page: ActivePage,
    pub main_page: MainPageState,
    pub setup: SetupState,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum ActivePage {
    #[default]
    Main,
    // Setup is comprised of multiple screens, handled in setup_page module
    Setup,
}
