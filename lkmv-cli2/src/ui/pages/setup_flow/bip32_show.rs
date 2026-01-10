use cli_clipboard::set_contents;
use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{
    COLOR_BORDER, COLOR_ORANGE, COLOR_SUCCESS, COLOR_TEXT_DEFAULT, COLOR_WARNING_ACCESSIBLE_RED,
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
use secrecy::ExposeSecret;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    state_handler::{
        actions::Action,
        setup_sequence::{BIP32PhraseAskChoice, BIP32PhraseShow, SetupState},
    },
    ui::{component::SetupFlowRender, pages::setup_flow::render_setup_header},
};

impl SetupFlowRender for BIP32PhraseShow {
    fn handle_key_event(
        &self,
        _state: &SetupState,
        action_tx: &mut UnboundedSender<Action>,
        key: KeyEvent,
    ) {
        match key.code {
            KeyCode::F(10) => {
                let _ = action_tx.send(Action::Exit);
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if set_contents(self.bip39_menemonic.get_mnemonic_string()).is_ok() {
                    let _ = action_tx.send(Action::SetupBIP32PhraseShowCopyToClipboard);
                }
            }
            KeyCode::Enter => {
                //let _ = action_tx.send(Action::SetupB(*self));
            }
            _ => {}
        }
    }

    fn render(&self, state: &SetupState, frame: &mut Frame) {
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
                "You must protect this seed phrase",
                Style::new().fg(COLOR_WARNING_ACCESSIBLE_RED).bold(),
            ),
            Line::styled(
                "Store it in a safe and secure location",
                Style::new().fg(COLOR_WARNING_ACCESSIBLE_RED).bold(),
            ),
            Line::default(),
            Line::styled(
                self.bip39_menemonic.get_mnemonic_string(),
                Style::new().fg(COLOR_SUCCESS),
            ),
        ];
        if self.clipboard_copied {
            lines.push(Line::styled(
                "Phrase copied to the clipboard!",
                Style::new().fg(COLOR_ORANGE).bold().slow_blink(),
            ));
        }

        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("[C]", Style::new().fg(COLOR_ORANGE).bold()),
            Span::styled(" Copy to clipboard  |  ", Style::new().fg(COLOR_ORANGE)),
            Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to continue", Style::new().fg(COLOR_BORDER)),
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
