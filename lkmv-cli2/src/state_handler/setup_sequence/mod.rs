// ****************************************************************************
// Setup Sequence Pages
// ****************************************************************************

use crate::state_handler::setup_sequence::bip32::BIP32_39;
use lkmv::config::PersonaDIDKeys;

pub mod bip32;
pub mod did_keys;

/// Setup flow has many pages, they are listed here
#[derive(Debug, Clone, Copy, Default)]
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
    DidKeysExportAsk,
    DidKeysExportInputs,
    DidKeysExportShow,
    UnlockCodeAsk,
    UnlockCodeSet,
    UnlockCodeWarn,
    MediatorAsk,
    MediatorCustom,
    UserName,
    WebVHAddress,
    FinalPage,
}

// ****************************************************************************
// State Management for the Setup Sequence
//
// All setup state is kept in a single struct
// ****************************************************************************

#[derive(Clone, Debug, Default)]
pub struct SetupState {
    pub active_page: SetupPage,

    /// BIP32 mnemonic to use
    pub mnemonic: BIP32_39,

    /// DID Keys
    pub did_keys: Option<PersonaDIDKeys>,

    pub did_keys_export: DIDKeysExportState,
}

/// Update messages as the Key export works through
#[derive(Clone, Debug, Default)]
pub struct DIDKeysExportState {
    pub messages: Vec<String>,
    pub exported: Option<String>,
}
