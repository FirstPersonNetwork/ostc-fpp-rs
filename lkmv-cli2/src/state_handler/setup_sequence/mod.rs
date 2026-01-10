// ****************************************************************************
// Setup Sequence Pages
// ****************************************************************************

use crate::{state_handler::actions::Action, ui::component::SetupFlowRender};
use crossterm::event::KeyEvent;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

/// Setup flow has many pages, they are listed here
#[derive(Clone, Copy, Debug, Default)]
pub enum SetupPage {
    #[default]
    StartAsk,
    ConfigImport, // Optional path where user will import existing config
    BIP32PhraseAsk,
    BIP32PhraseShow,
    BIP32PhraseImport,
    DIDKeysAsk,
    DIDKeysShow,
    DidKeysImport,
    DidKeysExport,
    MediatorAsk,
    MediatorCustomDID,
    DIDAddress,
    UserName,
}

impl SetupFlowRender for SetupPage {
    fn handle_key_event(
        &self,
        state: &SetupState,
        action_tx: &mut UnboundedSender<Action>,
        key: KeyEvent,
    ) {
        match self {
            SetupPage::StartAsk => state.start_ask.handle_key_event(state, action_tx, key),
            SetupPage::ConfigImport => state.config_import.handle_key_event(state, action_tx, key),
            SetupPage::BIP32PhraseAsk => state
                .bip32_phrase_ask
                .handle_key_event(state, action_tx, key),
            _ => {}
        }
    }

    fn render(&self, state: &SetupState, frame: &mut Frame) {
        match self {
            SetupPage::StartAsk => state.start_ask.render(state, frame),
            SetupPage::ConfigImport => state.config_import.render(state, frame),
            SetupPage::BIP32PhraseAsk => state.bip32_phrase_ask.render(state, frame),
            _ => {}
        }
    }
}

// ****************************************************************************
// State Management for the Setup Sequence
//
// There are two setup-state related structs:
// 1. SetupState - Contains state required by the UX
// 2. SetupBackendState - Contains additional sensitive state information only needed by the
//    backend
// All setup state is kept in a single struct
// ****************************************************************************

#[derive(Clone, Copy, Debug, Default)]
pub struct SetupState {
    pub active_page: SetupPage,
    pub start_ask: StartAskPanel,
    pub bip32_phrase_ask: BIP32PhraseAskChoice,
    pub config_import: ConfigImport,
}

// ****************************************************************************
// StartAsk
// ****************************************************************************
#[derive(Clone, Copy, Debug, Default)]
pub enum StartAskPanel {
    #[default]
    Create,
    Import,
}
impl StartAskPanel {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            StartAskPanel::Create => StartAskPanel::Import,
            StartAskPanel::Import => StartAskPanel::Create,
        }
    }
}

// ****************************************************************************
// Config Import
// ****************************************************************************
#[derive(Clone, Copy, Debug, Default)]
pub struct ConfigImport {}

// ****************************************************************************
// BIP32PhraseAsk
// ****************************************************************************
#[derive(Clone, Copy, Debug, Default)]
pub enum BIP32PhraseAskChoice {
    #[default]
    Create,
    Import,
}
impl BIP32PhraseAskChoice {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            BIP32PhraseAskChoice::Create => BIP32PhraseAskChoice::Import,
            BIP32PhraseAskChoice::Import => BIP32PhraseAskChoice::Create,
        }
    }
}
