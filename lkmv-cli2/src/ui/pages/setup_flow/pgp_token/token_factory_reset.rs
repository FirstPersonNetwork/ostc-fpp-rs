use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{COLOR_BORDER, COLOR_SUCCESS, COLOR_TEXT_DEFAULT, COLOR_WARNING_ACCESSIBLE_RED};
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
        setup_sequence::{MessageType, SetupPage, SetupState},
    },
    ui::pages::setup_flow::{SetupFlow, render_setup_header},
};

#[derive(Copy, Clone, Debug, Default)]
pub struct TokenFactoryReset {
    pub options: TokenFactoryResetOptions,

    /// Resetting and writing keys to the token
    pub writing_mode: bool,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum TokenFactoryResetOptions {
    #[default]
    Reset,
    NoReset,
}
impl TokenFactoryResetOptions {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            TokenFactoryResetOptions::Reset => TokenFactoryResetOptions::NoReset,
            TokenFactoryResetOptions::NoReset => TokenFactoryResetOptions::Reset,
        }
    }
}

impl TokenFactoryReset {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                state.token_factory_reset.options = state.token_factory_reset.options.switch();
            }
            KeyCode::Enter => {
                if state.token_factory_reset.writing_mode
                    && state.props.state.token_reset.completed_writing
                {
                    // Writing completed. Move to the next page
                    state.props.state.active_page = SetupPage::TokenSetTouch;
                } else if let TokenFactoryResetOptions::Reset = state.token_factory_reset.options {
                    if state.props.state.token_reset.completed_reset {
                        let _ = state.action_tx.send(Action::TokenWriteKeys(
                            state.token_select.selected_token.clone(),
                        ));
                    } else {
                        state.token_factory_reset.writing_mode = true;
                        let _ = state.action_tx.send(Action::FactoryReset(
                            state.token_select.selected_token.clone(),
                        ));
                    }
                } else {
                    state.token_factory_reset.writing_mode = true;
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
            .title(" Step 3/5: Factory Reset Token ");

        let mut lines = vec![
            Line::styled(
                "Would you like to do a factory reset your hardware token?",
                Style::new().fg(COLOR_BORDER).bold(),
            ),
            Line::default(),
            Line::styled(
                "Hardware Tokens can be in many possible states, causing unintended behaviour.",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::styled(
                "Resetting the token prior to writing your LKMV keys ensures a consistent and predictable state for the token.",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::default(),
        ];

        // Render the active chocie
        if let TokenFactoryResetOptions::Reset = self.options {
            lines.push(Line::styled(
                "[✓] Factory Reset (recommended)",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
            lines.push(Line::styled(
                "[ ] Do not reset token",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
        } else {
            lines.push(Line::styled(
                "[ ] Factory Reset (recommended)",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
            lines.push(Line::styled(
                "[✓] Do not reset token",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
        }

        lines.push(Line::default());

        if self.writing_mode {
            for msg in state.token_reset.messages.iter() {
                match msg {
                    MessageType::Info(info) => {
                        lines.push(Line::styled(
                            format!("INFO: {}", info),
                            Style::new().fg(COLOR_SUCCESS),
                        ));
                    }
                    MessageType::Error(err) => {
                        lines.push(Line::styled(
                            format!("ERROR: {}", err),
                            Style::new().fg(COLOR_WARNING_ACCESSIBLE_RED),
                        ));
                    }
                }
            }
            if state.token_reset.completed_reset {
                lines.push(Line::default());
                lines.push(Line::from(vec![
                    Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
                    Span::styled(" to continue", Style::new().fg(COLOR_TEXT_DEFAULT)),
                ]));
            }
        } else {
            lines.push(Line::from(vec![
                Span::styled("[TAB]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(" to select  |  ", Style::new().fg(COLOR_TEXT_DEFAULT)),
                Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(" to continue", Style::new().fg(COLOR_TEXT_DEFAULT)),
            ]));
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
