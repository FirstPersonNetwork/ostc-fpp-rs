use crate::{
    Interrupted, Terminator,
    state_handler::{
        actions::Action,
        main_page::MainPanel,
        setup_sequence::{BIP32PhraseAskChoice, SetupPage, StartAskPanel, bip32::BIP32_39},
        state::State,
    },
};
use anyhow::Result;
use lkmv::config::{Config, public_config::PublicConfig};
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
                let _ = terminator.terminate(Interrupted::SystemError);
                return Ok(Interrupted::SystemError);
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
                    Action::SetupStartAskPanelSwitch(choice_panel) => {
                            state.setup.start_ask = choice_panel;
                    }
                    Action::SetupStartAskSelectedPath(choice) => {
                        // User has chosen their setup starting path
                        match choice {
                            StartAskPanel::Create => state.setup.active_page = SetupPage::BIP32PhraseAsk,
                            StartAskPanel::Import => state.setup.active_page = SetupPage::ConfigImport,
                        }
                    }
                    Action::SetupBIP32PhraseAskChoiceSwitch(choice) => {
                            // User is selecting whether to create or import their BIP32 phrase
                        state.setup.bip32_phrase_ask = choice;
                    }
                    Action::SetupBIP32PhraseAskChoiceSelected(choice) => {
                        // User has chosen whether to create or import their BIP32 phrase
                        match choice {
                            BIP32PhraseAskChoice::Create => {
                                state.setup.active_page = SetupPage::BIP32PhraseShow;

                                // Create the new BIP32 seed and BIP39 phrase
                                state.setup.bip32_phrase_show.bip39_menemonic = BIP32_39::default();


                            },
                            BIP32PhraseAskChoice::Import => state.setup.active_page = SetupPage::BIP32PhraseImport,
                        }
                    }
                    Action::SetupBIP32PhraseShowCopyToClipboard => {
                        // Signal that the phrase has been copied to the clipboard
                        state.setup.bip32_phrase_show.clipboard_copied = true;
                    }
                    Action::SetupBIP32PhraseShowNext => {
                        // User has seen their BIP32 phrase, move to next setup step
                        state.setup.active_page = SetupPage::DIDKeysAsk;
                    }
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
