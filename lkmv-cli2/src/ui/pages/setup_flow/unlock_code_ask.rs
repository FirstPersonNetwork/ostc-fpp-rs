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
// UnlockCodeAsk
// ****************************************************************************
#[derive(Copy, Clone, Debug, Default)]
pub enum UnlockCodeAsk {
    #[default]
    UseCode,
    NoCode,
}
impl UnlockCodeAsk {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            UnlockCodeAsk::UseCode => UnlockCodeAsk::NoCode,
            UnlockCodeAsk::NoCode => UnlockCodeAsk::UseCode,
        }
    }
}

impl UnlockCodeAsk {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                state.unlock_code_ask = state.unlock_code_ask.switch();
            }
            KeyCode::Enter => {
                // User has chosen whether to create or import their BIP32 phrase
                state.props.state.active_page = match state.unlock_code_ask {
                    UnlockCodeAsk::UseCode => SetupPage::UnlockCodeSet,
                    UnlockCodeAsk::NoCode => SetupPage::UnlockCodeWarn,
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
            .title(" Unlock Code ");

        let mut lines = vec![
            Line::styled(
                "Use an unlock code to protect access to LKMV?",
                Style::new().fg(COLOR_BORDER).bold(),
            ),
            Line::default(),
            Line::styled(
                "As LKMV is a critical part of securing the overall Linux Community, this tool requires strong protection from unauthorized access.",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::styled(
                "It is strongly recommended to set an unlock passphrase that will be used to further protect access to this tool",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::default(),
        ];

        // Render the active chocie
        if let UnlockCodeAsk::UseCode = self {
            lines.push(Line::styled(
                "[✓] Yes, require unlock code when starting LKMV (recommended)",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
            lines.push(Line::styled(
                "[ ] No, keep unprotected",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
        } else {
            lines.push(Line::styled(
                "[ ] Yes, require unlock code when starting LKMV (recommended)",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
            lines.push(Line::styled(
                "[✓] No, keep unprotected",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
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
