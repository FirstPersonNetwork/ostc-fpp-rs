use crate::state_handler::{main_page::MainPageState, setup_page::SetupPageState};

/// State holds the state of the application
#[derive(Default, Debug, Clone)]
pub struct State {
    pub active_page: ActivePage,
    pub main_page: MainPageState,
    pub setup_page: SetupPageState,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum ActivePage {
    #[default]
    Main,

    // Setup is comprised of multiple screens, handled in setup_page module
    SetupChoice,                 // 1. Choose generate vs import
    SetupBIP32KeyInitialization, // Path 1.1 (Generate) BIP32 how do we generate the BIP 32 key? New or Immport?
    SetupImportBackup, // Path 1.2 (Import) BIP32 how do we generate the BIP 32 key? New or Immport?
}
