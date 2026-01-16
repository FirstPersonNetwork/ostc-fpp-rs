use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{
    COLOR_BORDER, COLOR_ORANGE, COLOR_SOFT_PURPLE, COLOR_TEXT_DEFAULT, COLOR_WARNING_ACCESSIBLE_RED,
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
                let mut ctx = ClipboardContext::new().unwrap();
                if ctx
                    .set_contents(state.props.state.mnemonic.get_mnemonic_string())
                    .is_ok()
                {
                    state.bip32_show.cc_copy = true;
                }
            }
            KeyCode::Enter => {
                state.props.state.active_page = SetupPage::DIDKeysAsk;
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
            .title(" Step 2/4: Save your recovery phrase ");

        let mut lines = vec![
            Line::styled(
                "Your BIP39 Recovery phrase (mnemonic of 24 words) below can be used to recover and regenerate your BIP32 based identity and security keys within LKMV.",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::default(),
            Line::styled(
                "You must protect this seed phrase. Store it in a safe and secure location",
                Style::new().fg(COLOR_WARNING_ACCESSIBLE_RED).bold(),
            ),
            Line::default(),
            Line::styled(
                state.mnemonic.get_mnemonic_string(),
                Style::new().fg(COLOR_SOFT_PURPLE).bold(),
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
                "Phrase copied to the clipboard!",
                Style::new().fg(COLOR_ORANGE).bold().slow_blink(),
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
