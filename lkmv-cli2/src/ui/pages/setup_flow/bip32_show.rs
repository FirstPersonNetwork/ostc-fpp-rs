use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{
    COLOR_BORDER, COLOR_ORANGE, COLOR_SOFT_PURPLE, COLOR_TEXT_DEFAULT, COLOR_DARK_GRAY, COLOR_SUCCESS,
};
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
        setup_sequence::{SetupState, did_keys::create_keys},
    },
    ui::pages::setup_flow::{SetupFlow, render_setup_header},
};

#[derive(Copy, Clone, Debug, Default)]
pub struct BIP32PhraseShow {
    /// Have we copied this to the clipboard?
    cc_copy: bool,
}

impl BIP32PhraseShow {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                let mut clipboard = Clipboard::new().unwrap();
                clipboard
                    .set_text(state.props.state.mnemonic.get_mnemonic_string())
                    .unwrap();
                state.bip32_show.cc_copy = true;
            }
            KeyCode::Enter => {
                // Create the DID Keys
                match create_keys(&state.props.state.mnemonic.mnemonic) {
                    Ok(keys) => {
                        let _ = state.action_tx.send(Action::SetDIDKeys(Box::new(keys)));
                    }
                    Err(e) => {
                        let _ = state
                            .action_tx
                            .send(Action::UXError(Interrupted::SystemError(format!(
                                "Failed to derive DID Keys: {}",
                                e
                            ))));
                    }
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
            .title(" Step 2/4: Save recovery phrase ");

        let mut lines = vec![
            Line::styled(
                "Your 24-word recovery phrase lets you restore your profile or set it up on another device using the same identity and security keys.",
                Style::new().fg(COLOR_DARK_GRAY),
            ),
            Line::styled(
                "This recovery phrase is a BIP39 mnemonic used to deterministically derive your BIP32 keys.",
                Style::new().fg(COLOR_DARK_GRAY),
            ),
            Line::default(),
            Line::styled(
                state.mnemonic.get_mnemonic_string(),
                Style::new().fg(COLOR_SOFT_PURPLE).bold(),
            ),
            Line::default(),
            Line::default(),
            Line::styled(
                "⚠️ Important Note: Keep this recovery phrase in a safe place, as it is required to restore your profile keys.",
                Style::new().fg(COLOR_ORANGE).bold(),
            ),
        ];

        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("[C]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(
                " Copy to clipboard  |  ",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to continue", Style::new().fg(COLOR_TEXT_DEFAULT)),
        ]));
        if self.cc_copy {
            lines.push(Line::styled(
                "Recovery phrase copied!",
                Style::new().fg(COLOR_SUCCESS).slow_blink(),
            ));
        }

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
