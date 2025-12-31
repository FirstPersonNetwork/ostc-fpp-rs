use crate::state_handler::{main_page::menu::MainMenu, state::MainPanel};

pub enum Action {
    Exit,
    /// A main menu item has been selected
    MainMenuSelected(MainMenu),

    /// Active Panel switched to
    MainPanelSwitch(MainPanel),
}
