use crate::{
    Interrupted,
    state_handler::main_page::{MainPanel, menu::MainMenu},
};

pub enum Action {
    Exit,

    /// An unrecoverable error has occurred on the UX Side
    UXError(Interrupted),

    /// A main menu item has been selected
    MainMenuSelected(MainMenu),

    /// Active Panel switched to
    MainPanelSwitch(MainPanel),
    // SETUP Pages
}
