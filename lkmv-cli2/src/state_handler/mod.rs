#[cfg(feature = "openpgp-card")]
use crate::state_handler::setup_sequence::openpgp_card::{
    set_cardholder_name, set_signing_touch_policy, write_keys_to_card,
};
use crate::{
    Interrupted, Terminator,
    state_handler::{
        actions::Action,
        main_page::MainPanel,
        setup_sequence::{
            Completion, ConfigProtection, MessageType, SetupPage, config::ConfigExtension,
            did_keys::export_persona_did_keys,
        },
        state::{ActivePage, State},
    },
};
use affinidi_tdk::{TDK, common::config::TDKConfig};
use anyhow::Result;
#[cfg(feature = "openpgp-card")]
use lkmv::openpgp_card::{factory_reset, get_cards};
use lkmv::{
    LF_PUBLIC_MEDIATOR_DID,
    bip32::get_bip32_root,
    config::{Config, did::create_initial_webvh_did, public_config::PublicConfig},
};
use pgp::composed::ArmorOptions;
use secrecy::SecretString;
use std::str::FromStr;
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

        // Instantiate TDK
        let tdk = TDK::new(
            TDKConfig::builder().with_load_environment(false).build()?,
            None,
        )
        .await?;

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
                    Action::ActivateMainMenu => {
                        // Switch to Main Menu
                        state.active_page = ActivePage::Main;
                        state.main_page.menu_panel.selected = true;
                        state.main_page.content_panel.selected = false;
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
                    Action::ImportConfig(filename, import_unlock_passphrase, new_unlock_passphrase) => {
                        // Import a configuration backup
                        let import_unlock_passphrase = SecretString::new(import_unlock_passphrase);
                        let new_unlock_passphrase = SecretString::new(new_unlock_passphrase);
                        state.setup.active_page = SetupPage::ConfigImport;
                        match Config::import(
                            &mut state, &self.state_tx,
                            &import_unlock_passphrase,
                            &new_unlock_passphrase,
                            &filename,
                            &self.profile,
                        ) {
                            Ok(()) => {
                                state.setup.config_import.completed = Completion::CompletedOK;
                                state.setup.config_import.messages.push(MessageType::Info("Configuration import completed successfully.".to_string()));
                            }
                            Err(e) => {
                                state.setup.config_import.messages.push(MessageType::Error(format!("Importing Config failed: {e}")));
                                state.setup.config_import.completed = Completion::CompletedFail;
                            }
                        }
                    },
                    Action::SetProtection(protection, next_page) => {
                        // Set the Config Protection method in setup state
                        state.setup.protection = protection;
                        state.setup.active_page = next_page;
                    },
                    Action::SetDIDKeys(persona_keys) => {
                        // Set the DID Persona Keys in setup state
                        state.setup.did_keys = Some(*persona_keys);
                        state.setup.active_page = SetupPage::DIDKeysShow;
                    },
                    Action::ExportDIDKeys(export_inputs) => {
                        // Handle exporting DID Keys
                        state.setup.active_page = SetupPage::DidKeysExportShow;
                        state.setup.did_keys_export.messages.push("Starting key export...".to_string());

                        // Send the intial state so that the UX shows the key export page
                        let _ = self.state_tx.send(state.clone());

                        let state_tx_clone = self.state_tx.clone();
                        let export = tokio::spawn(async move {
                         match export_persona_did_keys(&mut state, &state_tx_clone, export_inputs.username.value(), SecretString::from_str(export_inputs.passphrase.value()).unwrap()) {
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
                    #[cfg(feature = "openpgp-card")]
                    Action::GetTokens => {
                        // Fetch connected PGP Hardware Tokens
                        state.setup.active_page = SetupPage::TokenSelect;
                        match get_cards() {
                            Ok(cards) => {
                                state.setup.tokens.tokens = cards;
                            }
                            Err(e) => {
                                state.setup.tokens.messages = vec![format!("Error fetching tokens: {}", e)];
                            state.setup.tokens.tokens = vec![];
                            }
                        }
                    },
                    #[cfg(feature = "openpgp-card")]
                    Action::SetAdminPin(token, admin_pin) => {
                        state.setup.protection = ConfigProtection::Token(token);
                        state.token_admin_pin = Some(admin_pin);
                        state.setup.active_page = SetupPage::TokenFactoryReset;
                    }
                    #[cfg(feature = "openpgp-card")]
                    Action::FactoryReset(token) => {
                        if let Some(token) = token {
                            state.setup.token_reset.messages.push(MessageType::Info("Starting factory reset...".to_string()));
                            let reset = tokio::spawn(async move{match factory_reset(token) {
                                    Ok(_) => {
                                        state.setup.token_reset.messages.push(MessageType::Info("Factory reset completed successfully.".to_string()));
                                        state.setup.token_reset.completed_reset = true;
                                    },
                                    Err(e) => state.setup.token_reset.messages.push(MessageType::Error(format!("Factory reset failed: {}", e))),
                                }
                                state
                            }).await.unwrap();
                            state = reset;
                        } else {
                            state.setup.token_reset.messages.push(MessageType::Error("No token was specified.".to_string()));
                        }
                        state.setup.active_page = SetupPage::TokenFactoryReset;
                    }
                    #[cfg(feature = "openpgp-card")]
                    Action::TokenWriteKeys(token) => {
                        if let Some(token) = token {
                        let state_tx_clone = self.state_tx.clone();
                        let result = tokio::spawn(async move{match write_keys_to_card(&mut state, &state_tx_clone, token ) {
                             Ok(_) => {
                                    state.setup.token_reset.messages.push(MessageType::Info("Keys written to token successfully.".to_string()));
                                 state.setup.token_reset.completed_writing = true;
                             }
                             Err(e) => {
                                 state.setup.token_reset.messages.push(MessageType::Error(format!("Error writing keys to token: {}", e)));
                             }
                            }
                                state
                        }).await.unwrap();
                            state = result;
                        } else {
                            state.setup.token_reset.messages.push(MessageType::Error("No token was specified.".to_string()));
                        }
                    }
                    #[cfg(feature = "openpgp-card")]
                    Action::SetTouchPolicy(token) => {
                        // Called if enabling touch policy
                        state.setup.active_page = SetupPage::TokenSetTouch;
                        if let Some(token) = token {
                           match set_signing_touch_policy(&mut state, &self.state_tx, token) {
                                Ok(_) => state.setup.token_set_touch.completed = true,
                                Err(e) => {
                            state.setup.token_set_touch.messages.push(MessageType::Error(format!("An error occurred when setting touch policy: {e}")));
                                }
                            }
                        } else {
                            state.setup.token_set_touch.messages.push(MessageType::Error("No token was specified.".to_string()));
                        }
                            state.setup.token_set_touch.completed = true;
                    }
                    #[cfg(feature = "openpgp-card")]
                    Action::SetTokenName(token, name) => {
                        // Called if enabling touch policy
                        state.setup.active_page = SetupPage::TokenSetCardholderName;
                        if let Some(token) = token {
                           match set_cardholder_name(&mut state, &self.state_tx, token, &name) {
                                Ok(_) => state.setup.token_cardholder_name.completed = true,
                                Err(e) => {
                            state.setup.token_cardholder_name.messages.push(MessageType::Error(format!("An error occurred when setting cardholder name: {e}")));
                                }
                            }
                        } else {
                            state.setup.token_cardholder_name.messages.push(MessageType::Error("No token was specified.".to_string()));
                        }
                            state.setup.token_cardholder_name.completed = true;
                    }
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
                    Action::CreateWebVHDID(webvh_address) => {
                        // Set the WebVH DID in setup state
                        let mut keys = state.setup.did_keys.clone().unwrap();
                        match create_initial_webvh_did(&webvh_address, &mut keys, state.setup.custom_mediator.as_ref().unwrap_or(&LF_PUBLIC_MEDIATOR_DID.to_string()),
                        get_bip32_root(state.setup.mnemonic.mnemonic.to_entropy().as_slice()).unwrap()){
                            Ok((did, document)) => {
                                state.setup.webvh_address.did = did;
                                state.setup.webvh_address.document = document;
                                state.setup.did_keys = Some(keys);
                                state.setup.webvh_address.completed = Completion::CompletedOK;
                                state.setup.webvh_address.messages.push(MessageType::Info("WebVH DID created successfully.".to_string()));
                            },
                            Err(e) => {
                                state.setup.webvh_address.completed = Completion::CompletedFail;
                                state.setup.webvh_address.messages.push(MessageType::Error(format!("Error creating WebVH DID: {e}")));
                            }
                        }
                    },
                    Action::ResetWebVHDID => {
                        // Reset the WebVH DID state
                        state.setup.webvh_address.messages.clear();
                        state.setup.webvh_address.completed = Completion::NotFinished;
                    },
                    Action::ResolveWebVHDID(did) => {
                        // Check if can resolve DID
                        match tdk.did_resolver().resolve(&did).await {
                            Ok(response) => {
                                // Change the key ID's to match the DID VM ID's
                                if let Some(keys) = &mut state.setup.did_keys {
                                    keys.signing.secret.id = [&did, "#key-1"].concat();
                                    keys.authentication.secret.id = [&did, "#key-2"].concat();
                                    keys.decryption.secret.id = [&did, "#key-3"].concat();
                                }

                                state.setup.webvh_address.did = did;
                                state.setup.webvh_address.document = response.doc;
                                state.setup.webvh_address.completed = Completion::CompletedOK;
                                state.setup.webvh_address.messages.push(MessageType::Info("Your DID resolved successfully.".to_string()));
                            },
                            Err(e) => {
                                state.setup.webvh_address.completed = Completion::CompletedFail;
                                state.setup.webvh_address.messages.push(MessageType::Error(format!("Error resolving DID: {e}")));
                            }
                        }
                    }
                    Action::SetupCompleted(setup_flow) => {
                        // Final setup step completed
                        state.setup.active_page = SetupPage::FinalPage;
                        state.setup.final_page.messages.push(MessageType::Info("Generating your profile configuration...".to_string()));
                        state.setup.final_page.messages.push(MessageType::Info("Securing sensitive data for storage...".to_string()));
                        state.setup.final_page.messages.push(MessageType::Info("Your device may prompt for authentication to access OS secure storage.".to_string()));
                        self.state_tx.send(state.clone())?;
                        match Config::create(&state.setup, &setup_flow, &tdk, &self.profile).await {
                            Ok(_) => {
                                state.setup.final_page.completed = Completion::CompletedOK;
                                state.setup.final_page.messages.push(MessageType::Info("Profile setup completed successfully.".to_string()));
                            },
                            Err(e) => {
                                state.setup.final_page.completed = Completion::CompletedFail;
                                state.setup.final_page.messages.push(MessageType::Error(format!("Couldn't create LKMV configuration. Reason: {e}")));
                            }
                        }
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
