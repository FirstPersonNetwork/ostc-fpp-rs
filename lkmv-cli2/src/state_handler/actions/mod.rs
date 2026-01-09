use crate::state_handler::{
    main_page::{MainPanel, menu::MainMenu},
    setup_page::ChoicePanel,
    state::ActivePage,
};

pub enum Action {
    Exit,
    /// A main menu item has been selected
    MainMenuSelected(MainMenu),

    /// Active Panel switched to
    MainPanelSwitch(MainPanel),

    // SETUP Pages
    /// Active Panel switched to
    SetupChoicePanelSwitch(ChoicePanel),

    /// What starting path did the user select for setup?
    SetupChoiceSelectedPath(ActivePage),
}
