// ****************************************************************************
// Setup Sequence Pages
// ****************************************************************************

use std::fmt;
#[cfg(feature = "openpgp-card")]
use std::sync::Arc;

use crate::state_handler::setup_sequence::bip32::BIP32_39;
use lkmv::config::PersonaDIDKeys;
#[cfg(feature = "openpgp-card")]
use openpgp_card::{Card, state::Open};
#[cfg(feature = "openpgp-card")]
use tokio::sync::Mutex;

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

    /// Optional PGP Token setup occurs here
    #[cfg(feature = "openpgp-card")]
    TokenStart,
    #[cfg(feature = "openpgp-card")]
    TokenSelect,
    #[cfg(feature = "openpgp-card")]
    TokenFactoryReset,
    #[cfg(feature = "openpgp-card")]
    TokenSetTouch,
    #[cfg(feature = "openpgp-card")]
    TokenSetCardholderName,

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

#[derive(Clone, Default, Debug)]
pub struct SetupState {
    pub active_page: SetupPage,

    /// BIP32 mnemonic to use
    pub mnemonic: BIP32_39,

    /// DID Keys
    pub did_keys: Option<PersonaDIDKeys>,

    /// Contains the PGP formatted export of DID keys if user selected to export
    pub did_keys_export: DIDKeysExportState,

    /// PGP Hardware Tokens that are connected
    #[cfg(feature = "openpgp-card")]
    pub tokens: DetectedTokens,

    /// Has the user selected to use a custom Mediator?
    pub custom_mediator: Option<String>,

    /// What username is the user using?
    pub username: String,

    /// What address to sue for WebVH?
    pub webvh_address: String,
}

/// Update messages as the Key export works through
#[derive(Clone, Debug, Default)]
pub struct DIDKeysExportState {
    pub messages: Vec<String>,
    pub exported: Option<String>,
}

/// State relating to detecting attached hardware tokens
#[cfg(feature = "openpgp-card")]
#[derive(Clone, Default)]
pub struct DetectedTokens {
    pub tokens: Vec<Arc<Mutex<Card<Open>>>>,
    pub messages: Vec<String>,
}

#[cfg(feature = "openpgp-card")]
impl fmt::Debug for DetectedTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DetectedTokens {{ tokens: {}, messages: {:?} }}",
            self.tokens.len(),
            self.messages
        )
    }
}
