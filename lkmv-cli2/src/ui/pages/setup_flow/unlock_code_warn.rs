use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{
    COLOR_BORDER, COLOR_ORANGE, COLOR_SOFT_PURPLE, COLOR_SUCCESS, COLOR_TEXT_DEFAULT,
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
// UnlockCodeWarn
// ****************************************************************************
#[derive(Copy, Clone, Debug, Default)]
pub enum UnlockCodeWarn {
    #[default]
    UseCode,
    AckRisk,
}
impl UnlockCodeWarn {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            UnlockCodeWarn::UseCode => UnlockCodeWarn::AckRisk,
            UnlockCodeWarn::AckRisk => UnlockCodeWarn::UseCode,
        }
    }
}
impl UnlockCodeWarn {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                state.unlock_code_warn = state.unlock_code_warn.switch();
            }
            KeyCode::Enter => {
                // User has chosen whether to create or import their BIP32 phrase
                state.props.state.active_page = match state.unlock_code_warn {
                    UnlockCodeWarn::UseCode => SetupPage::UnlockCodeSet,
                    UnlockCodeWarn::AckRisk => SetupPage::MediatorAsk,
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
            .fg(COLOR_ORANGE)
            .padding(Padding::proportional(1))
            .title(" Unlock Code WARNING ");

        let mut lines = vec![
            Line::styled(
                "⚠ SECURITY WARNING:",
                Style::new().fg(COLOR_WARNING_ACCESSIBLE_RED).bold(),
            ),
            Line::default(),
            Line::styled(
                "You have chosen NOT to use an unlock code.",
                Style::new().fg(COLOR_ORANGE),
            ),
            Line::default(),
            Line::from(vec![
                Span::styled("📋 Storage: ", Style::new().fg(COLOR_SOFT_PURPLE).bold()),
                Span::styled(
                    "Your keys will be stored as Base64-encoded plain text!",
                    Style::new().fg(COLOR_ORANGE),
                ),
            ]),
            Line::default(),
            Line::from(vec![
                Span::styled("⚡ Risk: ", Style::new().fg(COLOR_SOFT_PURPLE).bold()),
                Span::styled(
                    "Anyone with access to your device can easily decode and use your keys without any protection.",
                    Style::new().fg(COLOR_ORANGE),
                ),
            ]),
            Line::default(),
            Line::styled(
                "This is not recommended for production use.",
                Style::new().fg(COLOR_ORANGE),
            ),
            Line::default(),
            Line::styled(
                "Do you accept this risk?",
                Style::new().fg(COLOR_BORDER).bold(),
            ),
            Line::default(),
        ];

        // Render the active chocie
        if let UnlockCodeWarn::UseCode = self {
            lines.push(Line::styled(
                "[✓] No, I want to set an unlock code (recommended)",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
            lines.push(Line::styled(
                "[ ] I know what I am doing, I accept this risk",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
        } else {
            lines.push(Line::styled(
                "[ ] No, I want to set an unlock code (recommended)",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
            lines.push(Line::styled(
                "[✓] I know what I am doing, I accept this risk",
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
