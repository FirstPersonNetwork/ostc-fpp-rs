use std::str::FromStr;

use crate::{
    Interrupted, Terminator,
    state_handler::{
        actions::Action, main_page::MainPanel, setup_sequence::SetupPage, state::State,
    },
    ui::pages::setup_flow::did_keys_ask::DIDKeysAsk,
};
use anyhow::Result;
use lkmv::config::{Config, public_config::PublicConfig};
use pgp::composed::ArmorOptions;
use secrecy::SecretString;
use tokio::sync::{
    broadcast,
    mpsc::{self, UnboundedReceiver, UnboundedSender},
};
use tracing::error;

pub mod actions;
pub mod main_page;
pub mod setup_sequence;
pub mod state;

pub struct StateHandler {
    state_tx: UnboundedSender<State>,
    profile: String,
}

impl StateHandler {
    pub fn new(profile: &str) -> (Self, UnboundedReceiver<State>) {
        let (state_tx, state_rx) = mpsc::unbounded_channel::<State>();

        (
            StateHandler {
                state_tx,
                profile: profile.to_string(),
            },
            state_rx,
        )
    }

    pub async fn main_loop(
        self,
        mut terminator: Terminator,
        mut action_rx: UnboundedReceiver<Action>,
        mut interrupt_rx: broadcast::Receiver<Interrupted>,
    ) -> Result<Interrupted> {
        let mut state = State::default();

        let public_config = match Config::load_step1(&self.profile) {
            Ok(pc) => pc,
            Err(lkmv::errors::LKMVError::ConfigNotFound(_, _)) => {
                // Configuration not found, start in setup mode
                state.active_page = state::ActivePage::Setup;
                PublicConfig::default()
            }
            Err(e) => {
                error!("Couldn't load configuration step1: {e}");
                let err = Interrupted::SystemError(format!("Couldn't load configuration: {e}"));
                let _ = terminator.terminate(err.clone());
                return Ok(err);
            }
        };

        // Send the initial state once
        self.state_tx.send(state.clone())?;

        let result = loop {
            tokio::select! {
                Some(action) = action_rx.recv() => match action {
                    Action::Exit => {
                        let _ = terminator.terminate(Interrupted::UserInt);

                        break Interrupted::UserInt;
                    },
                    Action::UXError(interrupted) => {
                        // An error has occurred on the UX side
                        let _ = terminator.terminate(interrupted.clone());

                        break interrupted;
                    },
                    Action::MainMenuSelected(menu_item) => {
                        // User has changed main menu selection
                        state.main_page.menu_panel.selected_menu = menu_item;
                    },
                    Action::MainPanelSwitch(panel) => {
                        match panel {
                            MainPanel::ContentPanel => {
                                // When switching to ContentPanel, reset any content-specific state if needed
                                state.main_page.menu_panel.selected = false;
                                state.main_page.content_panel.selected = true;
                            },
                            MainPanel::MainMenu => {
                                // When switching to MainMenu, reset any content-specific state if needed
                                state.main_page.menu_panel.selected = true;
                                state.main_page.content_panel.selected = false;
                            }
                        }
                    },
                    Action::ExportDIDKeys(persona_keys, export_inputs) => {
                        // Handle exporting DID Keys
                        state.setup.active_page = SetupPage::DidKeysExportShow;
                        state.setup.did_keys = Some(*persona_keys);
                        state.setup.did_keys_export.messages.push("Starting key export...".to_string());

                        // Send the intial state so that the UX shows the key export page
                        let _ = self.state_tx.send(state.clone());

                        let state_tx_clone = self.state_tx.clone();
                        let export = tokio::spawn(async move {
                         match DIDKeysAsk::export_persona_did_keys(&mut state, &state_tx_clone, export_inputs.username.value(), SecretString::from_str(export_inputs.passphrase.value()).unwrap()) {
                            Ok(export) => {
                                state.setup.did_keys_export.exported =  match export.to_armored_string(ArmorOptions::default()) {
                                    Ok(armored) => Some(armored),
                                    Err(e) => {
                                            state.setup.did_keys_export.messages.push(format!("Error armoring exported keys: {}", e));
                                            None
                                    }
                                };
                            }
                            Err(e) => {
                                    state.setup.did_keys_export.messages.push(format!("Error exporting DID keys: {}", e));
                            }

                        }
                            state
                        }).await.unwrap();
                        state = export;
                        if state.setup.did_keys_export.exported.is_some() {
                            state.setup.did_keys_export.messages.push("Key export completed".to_string());
                        }
                    },
                    Action::SetCustomMediator(mediator_did) => {
                        // Set the Custom Mediator in setup state
                        state.setup.custom_mediator = Some(mediator_did);
                        state.setup.active_page = SetupPage::UserName;
                    }
                    Action::SetUsername(username) => {
                        // Set the username in setup state
                        state.setup.username = username;
                        state.setup.active_page = SetupPage::WebVHAddress;
                    },
                    Action::SetupCompleted(webvh_address) => {
                        // Final setup step completed
                        state.setup.webvh_address = webvh_address;
                        state.setup.active_page = SetupPage::FinalPage;
                    },
                },
                // Catch and handle interrupt signal to gracefully shutdown
                Ok(interrupted) = interrupt_rx.recv() => {
                    break interrupted;
                }
            }
            self.state_tx.send(state.clone())?;
        };

        Ok(result)
    }
}
