// ****************************************************************************
// Setup Sequence Pages
// ****************************************************************************

use crate::state_handler::setup_sequence::bip32::BIP32_39;
#[cfg(feature = "openpgp-card")]
use ::openpgp_card::{Card, state::Open};
use lkmv::config::PersonaDIDKeys;
use std::fmt;
#[cfg(feature = "openpgp-card")]
use std::sync::Arc;
#[cfg(feature = "openpgp-card")]
use tokio::sync::Mutex;

pub mod bip32;
pub mod config;
pub mod did_keys;
#[cfg(feature = "openpgp-card")]
pub mod openpgp_card;

/// Setup flow has many pages, they are listed here
#[derive(Debug, Clone, Copy, Default)]
pub enum SetupPage {
    #[default]
    StartAsk,
    ConfigImport, // Optional path where user will import existing config
    BIP32PhraseAsk,
    BIP32PhraseShow,
    BIP32PhraseImport,
    DIDKeysShow,
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

    pub config_import: ConfigImport,

    /// BIP32 mnemonic to use
    pub mnemonic: BIP32_39,

    /// DID Keys
    pub did_keys: Option<PersonaDIDKeys>,

    /// Contains the PGP formatted export of DID keys if user selected to export
    pub did_keys_export: DIDKeysExportState,

    /// PGP Hardware Tokens that are connected
    #[cfg(feature = "openpgp-card")]
    pub tokens: DetectedTokens,

    /// Hardware Token Reset State
    #[cfg(feature = "openpgp-card")]
    pub token_reset: FactoryResetToken,

    /// Hardware Touch Policy
    #[cfg(feature = "openpgp-card")]
    pub token_set_touch: TokenSetTouchPolicy,

    /// Hardware Cardholder Name
    #[cfg(feature = "openpgp-card")]
    pub token_cardholder_name: TokenSetCardholderName,

    /// Has the user selected to use a custom Mediator?
    pub custom_mediator: Option<String>,

    /// What username is the user using?
    pub username: String,

    /// What address to sue for WebVH?
    pub webvh_address: String,
}

/// Helps format messages from backend to the frontend
#[derive(Clone, Debug)]
pub enum MessageType {
    Info(String),
    Error(String),
}

/// Completion States for tasks
#[derive(Clone, Debug, Default)]
pub enum Completion {
    #[default]
    NotFinished,
    CompletedOK,
    CompletedFail,
}

/// State relating to importing configuration
#[derive(Clone, Default, Debug)]
pub struct ConfigImport {
    pub completed: Completion,
    pub messages: Vec<MessageType>,
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

/// State relating to factory reset of hardware token
/// Also contains writing keys to the token
#[cfg(feature = "openpgp-card")]
#[derive(Clone, Default, Debug)]
pub struct FactoryResetToken {
    pub completed_reset: bool,
    pub completed_writing: bool,
    pub messages: Vec<MessageType>,
}

/// State relating to token touch policy
#[cfg(feature = "openpgp-card")]
#[derive(Clone, Default, Debug)]
pub struct TokenSetTouchPolicy {
    pub completed: bool,
    pub messages: Vec<MessageType>,
}

/// State relating to token cardholder name
#[cfg(feature = "openpgp-card")]
#[derive(Clone, Default, Debug)]
pub struct TokenSetCardholderName {
    pub completed: bool,
    pub messages: Vec<MessageType>,
}
