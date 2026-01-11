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
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    state_handler::{
        actions::Action,
        setup_sequence::{BIP32PhraseAskChoice, SetupState},
    },
    ui::{component::SetupFlowRender, pages::setup_flow::render_setup_header},
};

impl SetupFlowRender for BIP32PhraseAskChoice {
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
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                // Switch active panel
                let _ = action_tx.send(Action::SetupBIP32PhraseAskChoiceSwitch(self.switch()));
            }
            KeyCode::Enter => {
                let _ = action_tx.send(Action::SetupBIP32PhraseAskChoiceSelected(*self));
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
