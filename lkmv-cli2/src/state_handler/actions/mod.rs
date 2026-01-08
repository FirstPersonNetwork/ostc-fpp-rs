use crate::state_handler::main_page::{MainPanel, menu::MainMenu};

pub enum Action {
    Exit,
    /// A main menu item has been selected
    MainMenuSelected(MainMenu),

    /// Active Panel switched to
    MainPanelSwitch(MainPanel),
}
