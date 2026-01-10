use crate::state_handler::{
    main_page::{MainPanel, menu::MainMenu},
    setup_sequence::{BIP32PhraseAskChoice, StartAskPanel},
};

pub enum Action {
    Exit,
    /// A main menu item has been selected
    MainMenuSelected(MainMenu),

    /// Active Panel switched to
    MainPanelSwitch(MainPanel),

    // SETUP Pages
    /// Active Panel switched to
    SetupStartAskPanelSwitch(StartAskPanel),

    /// What starting path did the user select for setup?
    SetupStartAskSelectedPath(StartAskPanel),

    SetupBIP32PhraseAskChoiceSwitch(BIP32PhraseAskChoice),
}
