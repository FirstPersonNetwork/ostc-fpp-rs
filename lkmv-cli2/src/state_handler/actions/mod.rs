#[cfg(feature = "openpgp-card")]
use std::sync::Arc;

use lkmv::config::PersonaDIDKeys;
#[cfg(feature = "openpgp-card")]
use openpgp_card::{Card, state::Open};
#[cfg(feature = "openpgp-card")]
use secrecy::SecretString;
#[cfg(feature = "openpgp-card")]
use tokio::sync::Mutex;

use crate::{
    Interrupted,
    state_handler::main_page::{MainPanel, menu::MainMenu},
    ui::pages::setup_flow::did_keys_export_inputs::DIDKeysExportInputs,
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
    /// Sets the DID Persona Keys
    SetDIDKeys(Box<PersonaDIDKeys>),

    /// Export DID Private keys as PGP Armored file
    ExportDIDKeys(DIDKeysExportInputs),

    /// Fetches PGP Hardware Tokens that are connected
    #[cfg(feature = "openpgp-card")]
    GetTokens,

    /// Set the Admin PIN Code for the Hardware Token
    #[cfg(feature = "openpgp-card")]
    SetAdminPin(SecretString),

    /// Set the Touch Policy
    #[cfg(feature = "openpgp-card")]
    SetTouchPolicy(Option<Arc<Mutex<Card<Open>>>>),

    /// Set the Cardholdername
    #[cfg(feature = "openpgp-card")]
    SetTokenName(Option<Arc<Mutex<Card<Open>>>>, String),

    /// Factory Reset Hardware Token
    #[cfg(feature = "openpgp-card")]
    FactoryReset(Option<Arc<Mutex<Card<Open>>>>),

    /// Write Keys
    #[cfg(feature = "openpgp-card")]
    TokenWriteKeys(Option<Arc<Mutex<Card<Open>>>>),

    /// Using a custom mediator DID
    SetCustomMediator(String),

    /// What username to be known as
    SetUsername(String),

    /// Final setup step completed, send the WEBVH Hosting Address
    SetupCompleted(String),
}
