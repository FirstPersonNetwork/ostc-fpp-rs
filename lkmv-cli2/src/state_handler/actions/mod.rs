use crate::state_handler::state::MainMenu;

pub enum Action {
    Exit,
    /// A main menu item has been selected
    MainMenuSelected(MainMenu),
}
