use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{COLOR_BORDER, COLOR_SUCCESS, COLOR_TEXT_DEFAULT};
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
// BIP32PhraseAsk
// ****************************************************************************
#[derive(Copy, Clone, Debug, Default)]
pub enum BIP32PhraseAskChoice {
    #[default]
    Create,
    Import,
}
impl BIP32PhraseAskChoice {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            BIP32PhraseAskChoice::Create => BIP32PhraseAskChoice::Import,
            BIP32PhraseAskChoice::Import => BIP32PhraseAskChoice::Create,
        }
    }
}

impl BIP32PhraseAskChoice {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                state.props.bip32_ask = state.props.bip32_ask.switch();
            }
            KeyCode::Enter => {
                // User has chosen whether to create or import their BIP32 phrase
                state.props.state.active_page = match state.props.bip32_ask {
                    BIP32PhraseAskChoice::Create => SetupPage::BIP32PhraseShow,
                    BIP32PhraseAskChoice::Import => SetupPage::BIP32PhraseImport,
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
            .title(" Step 1/4: BIP32 Seed Phrase ");

        let mut lines = vec![
            Line::styled(
                "LKMV derives individual keys from a common BIP32 seed phrase. This allows for a secure and private deterministic generation of key material from a single seed, rather than having to back up and restore seed material for every key.",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::default(),
            Line::styled(
                "Choose how to setup your BIP32 recovery phrase",
                Style::new().fg(COLOR_BORDER).bold(),
            ),
            Line::default(),
        ];

        // Render the active chocie
        if let BIP32PhraseAskChoice::Create = self {
            lines.push(Line::styled(
                "[✓] Generate a new 24-word recovery phrase (recommended)",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
            lines.push(Line::styled(
                "[ ] Import an existing recovery phrase",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
        } else {
            lines.push(Line::styled(
                "[ ] Generate a new 24-word recovery phrase (recommended)",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
            lines.push(Line::styled(
                "[✓] Import an existing recovery phrase",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
        }

        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("[TAB]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to select  |  ", Style::new().fg(COLOR_BORDER)),
            Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to confirm", Style::new().fg(COLOR_BORDER)),
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
