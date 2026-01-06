use crate::state_handler::main_page::{content::ContentPanelState, menu::MenuPanelState};

pub mod content;
pub mod menu;

/// Holds all state related info for the main page
#[derive(Clone, Debug, Default)]
pub struct MainPageState {
    /// State related to the menu panel
    pub menu_panel: MenuPanelState,

    /// State related to the content panel
    pub content_panel: ContentPanelState,
}
