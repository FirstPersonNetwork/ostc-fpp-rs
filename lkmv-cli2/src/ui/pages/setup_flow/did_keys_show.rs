use cli_clipboard::set_contents;
use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{
    COLOR_BORDER, COLOR_DARK_PURPLE, COLOR_ORANGE, COLOR_SUCCESS, COLOR_TEXT_DEFAULT,
    COLOR_WARNING_ACCESSIBLE_RED,
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
    state_handler::{
        actions::Action,
        setup_sequence::{SetupPage, SetupState},
    },
    ui::pages::setup_flow::{SetupFlow, render_setup_header},
};

// ****************************************************************************
// DIDKeysShow
// ****************************************************************************
#[derive(Copy, Clone, Debug, Default)]
pub struct DIDKeysShow {
    /// 0 = Nothing copied to clipboard
    /// 1 = Signing Key copied
    /// 2 = Authentication Key copied
    /// 3 = Encryption Key copied
    pub show_clipboard_copy: u8,
}

impl DIDKeysShow {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Char('1') => {
                if let Some(did_keys) = &state.props.state.did_keys
                    && set_contents(did_keys.signing.secret.get_public_keymultibase().unwrap())
                        .is_ok()
                {
                    state.did_keys_show.show_clipboard_copy = 1;
                }
            }
            KeyCode::Char('2') => {
                if let Some(did_keys) = &state.props.state.did_keys
                    && set_contents(
                        did_keys
                            .authentication
                            .secret
                            .get_public_keymultibase()
                            .unwrap(),
                    )
                    .is_ok()
                {
                    state.did_keys_show.show_clipboard_copy = 2;
                }
            }
            KeyCode::Char('3') => {
                if let Some(did_keys) = &state.props.state.did_keys
                    && set_contents(
                        did_keys
                            .decryption
                            .secret
                            .get_public_keymultibase()
                            .unwrap(),
                    )
                    .is_ok()
                {
                    state.did_keys_show.show_clipboard_copy = 3;
                }
            }
            KeyCode::Enter => {
                state.props.state.active_page = SetupPage::DidKeysExport;
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
            .title(" Step 3/4: Derived DID Keys ");

        let mut lines = vec![
            Line::from(vec![
                Span::styled("✓ ", Style::new().fg(COLOR_SUCCESS).bold()),
                Span::styled(
                    "The following keys were successfully derived from your BIP32 seed:",
                    Style::new().fg(COLOR_BORDER).bold(),
                ),
            ]),
            Line::default(),
        ];

        // Render the keys
        if let Some(did_keys) = &state.did_keys {
            // Signing Key
            lines.push(Line::from(vec![
                Span::styled("Signing Key ", Style::new().fg(COLOR_ORANGE).bold()),
                Span::styled(
                    format!("({}) created:", did_keys.signing.secret.get_key_type()),
                    Style::new().fg(COLOR_BORDER),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("🔑 ", Style::new()),
                Span::styled(
                    did_keys.signing.secret.get_public_keymultibase().unwrap(),
                    Style::new().fg(COLOR_DARK_PURPLE),
                ),
                Span::styled("  [1]", Style::new().fg(COLOR_BORDER)).bold(),
                Span::styled(" to copy", Style::new().fg(COLOR_TEXT_DEFAULT)),
                if self.show_clipboard_copy == 1 {
                    Span::styled(
                        "  (copied!)",
                        Style::new().fg(COLOR_SUCCESS).bold().slow_blink(),
                    )
                } else {
                    Span::styled("", Style::new())
                },
            ]));
            lines.push(Line::default());

            // Authentication Key
            lines.push(Line::from(vec![
                Span::styled("Authentication Key ", Style::new().fg(COLOR_ORANGE).bold()),
                Span::styled(
                    format!(
                        "({}) created:",
                        did_keys.authentication.secret.get_key_type()
                    ),
                    Style::new().fg(COLOR_BORDER),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("🔑 ", Style::new()),
                Span::styled(
                    did_keys
                        .authentication
                        .secret
                        .get_public_keymultibase()
                        .unwrap(),
                    Style::new().fg(COLOR_DARK_PURPLE),
                ),
                Span::styled("  [2]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(" to copy", Style::new().fg(COLOR_TEXT_DEFAULT)),
                if self.show_clipboard_copy == 2 {
                    Span::styled(
                        "  (copied!)",
                        Style::new().fg(COLOR_SUCCESS).bold().slow_blink(),
                    )
                } else {
                    Span::styled("", Style::new())
                },
            ]));
            lines.push(Line::default());

            // Decryption Key
            lines.push(Line::from(vec![
                Span::styled("Decryption Key ", Style::new().fg(COLOR_ORANGE).bold()),
                Span::styled(
                    format!("({}) created:", did_keys.decryption.secret.get_key_type()),
                    Style::new().fg(COLOR_BORDER),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("🔑 ", Style::new()),
                Span::styled(
                    did_keys
                        .decryption
                        .secret
                        .get_public_keymultibase()
                        .unwrap(),
                    Style::new().fg(COLOR_DARK_PURPLE),
                ),
                Span::styled("  [3]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(" to copy", Style::new().fg(COLOR_TEXT_DEFAULT)),
                if self.show_clipboard_copy == 3 {
                    Span::styled(
                        "  (copied!)",
                        Style::new().fg(COLOR_SUCCESS).bold().slow_blink(),
                    )
                } else {
                    Span::styled("", Style::new())
                },
            ]));
            lines.push(Line::default());
        } else {
            lines.push(Line::from(vec![
                Span::styled(
                    "ERROR: ",
                    Style::new().fg(COLOR_WARNING_ACCESSIBLE_RED).bold(),
                ),
                Span::styled(
                    "Expected to see DID Keys, instead they don't exist!",
                    Style::new().fg(COLOR_ORANGE),
                ),
            ]));
        }

        lines.push(Line::from(vec![Span::styled("NOTE: ", Style::new().fg(COLOR_ORANGE).bold()), Span::styled("You can export these keys for use in other applications from within LKMV at any point in the future.", Style::new().fg(COLOR_BORDER))]));
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(
                " Continue to next step",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
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
