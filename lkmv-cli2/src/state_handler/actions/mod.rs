use lkmv::config::PersonaDIDKeys;

use crate::{
    Interrupted,
    state_handler::main_page::{MainPanel, menu::MainMenu},
    ui::pages::setup_flow::{SetupFlow, did_keys_export_inputs::DIDKeysExportInputs},
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
    /// Export DID Private keys as PGP Armored file
    ExportDIDKeys(Box<PersonaDIDKeys>, DIDKeysExportInputs),

    /// Using a custom mediator DID
    SetCustomMediator(String),

    /// What username to be known as
    SetUsername(String),

    /// Final setup step completed, send the WEBVH Hosting Address
    SetupCompleted(String),
}
