use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{COLOR_BORDER, COLOR_ORANGE, COLOR_SUCCESS, COLOR_TEXT_DEFAULT};
use ratatui::{
    Frame,
    layout::{
        Constraint::{Length, Min},
        Layout,
    },
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph, Wrap},
};

use crate::{
    Interrupted,
    state_handler::{
        actions::Action,
        setup_sequence::{SetupPage, SetupState},
    },
    ui::pages::setup_flow::{SetupFlow, render_setup_header},
};

// ****************************************************************************
// DIDKeysAsk
// ****************************************************************************
#[derive(Copy, Clone, Debug, Default)]
pub enum DIDKeysAsk {
    #[default]
    Create,
    Import,
}
impl DIDKeysAsk {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            DIDKeysAsk::Create => DIDKeysAsk::Import,
            DIDKeysAsk::Import => DIDKeysAsk::Create,
        }
    }
}

impl DIDKeysAsk {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                state.did_keys_ask = state.did_keys_ask.switch();
            }
            KeyCode::Enter => {
                match state.did_keys_ask {
                    DIDKeysAsk::Create => {
                        // Create the DID Keys
                        match DIDKeysAsk::create_keys(&state.props.state.mnemonic.mnemonic) {
                            Ok(keys) => {
                                let _ = state.action_tx.send(Action::SetDIDKeys(Box::new(keys)));
                            }
                            Err(e) => {
                                let _ = state.action_tx.send(Action::UXError(
                                    Interrupted::SystemError(format!(
                                        "Failed to derive DID Keys: {}",
                                        e
                                    )),
                                ));
                            }
                        }
                    }
                    DIDKeysAsk::Import => state.props.state.active_page = SetupPage::DidKeysImport,
                }
            }
            _ => {}
        }
    }

    pub fn render(&self, state: &SetupState, frame: &mut Frame) {
        let [top, middle, bottom] =
            Layout::vertical([Length(3), Min(0), Length(3)]).areas(frame.area());

        render_setup_header(frame, top, state);

        let block = Block::bordered()
            .fg(COLOR_BORDER)
            .padding(Padding::proportional(1))
            .title(" Step 3/4: DID Key Derivation ");

        let mut lines = vec![
            Line::styled(
                "How should LKMV create your keys?",
                Style::new().fg(COLOR_BORDER).bold(),
            ),
            Line::default(),
            Line::styled(
                "LKMV will create a Decentralized Identifier (DID) for you. This DID provides both a globally unique identifier as well as presenting your public-key infrastructure (PKI) to others for authorisation and encryption.",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::default(),
            Line::styled(
                "Your DID will contain a number of keys that allow you to assert claims and authenticate as yourself with others. These keys can be automatically derived from your BIP32 phrase.",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::default(),
        ];

        // Render the active chocie
        if let DIDKeysAsk::Create = self {
            lines.push(Line::styled(
                "[✓] Derive new keys from recovery phrase (recommended)",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
            lines.push(Line::styled(
                "[ ] Import existing PGP Keys",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
        } else {
            lines.push(Line::styled(
                "[ ] Derive new keys from recovery phrase (recommended)",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
            lines.push(Line::styled(
                "[✓] Import existing PGP Keys",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
            lines.push(Line::from(vec![
                Span::styled("ADVANCED:", Style::new().fg(COLOR_ORANGE).bold()),
                Span::styled(
                    " You can choose to import existing PGP keys (Must be Curve 25519 based) instead of deriving them from your BIP32 phrase.",
                    Style::new().fg(COLOR_ORANGE),
                ),
            ]));
        }

        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("[TAB]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to select  |  ", Style::new().fg(COLOR_TEXT_DEFAULT)),
            Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to confirm", Style::new().fg(COLOR_TEXT_DEFAULT)),
        ]));

        frame.render_widget(
            Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
            middle,
        );

        let bottom_line = Line::from(vec![
            Span::styled("[F10]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to quit", Style::new().fg(COLOR_TEXT_DEFAULT)),
        ]);

        frame.render_widget(
            Paragraph::new(bottom_line).block(Block::new().padding(Padding::new(2, 0, 1, 0))),
            bottom,
        );
    }
}
