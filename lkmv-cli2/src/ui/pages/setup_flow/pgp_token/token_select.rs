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
    state_handler::{actions::Action, setup_sequence::SetupState},
    ui::pages::setup_flow::{SetupFlow, render_setup_header},
};

#[derive(Copy, Clone, Debug, Default)]
pub struct TokenSelect {
    pub selected: usize,
}

impl TokenSelect {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Down => {
                let token_count = state.props.state.tokens.tokens.len();
                state.token_select.selected = (state.token_select.selected + 1) % (token_count + 1);
            }
            KeyCode::Up => {
                let token_count = state.props.state.tokens.tokens.len();
                if state.token_select.selected == 0 {
                    state.token_select.selected = token_count;
                } else {
                    state.token_select.selected -= 1;
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                let _ = state.action_tx.send(Action::GetTokens);
            }
            KeyCode::Enter => {
                // Get Admin PIN
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
            .title(" Step 1/4: Select Hardware Token ");

        let mut lines = Vec::new();

        if !state.tokens.messages.is_empty() {
            for msg in state.tokens.messages.iter() {
                lines.push(Line::styled(
                    format!("ERROR: {msg}"),
                    Style::new().fg(COLOR_WARNING_ACCESSIBLE_RED).italic(),
                ));
            }
            lines.push(Line::default());
        }

        if state.tokens.tokens.is_empty() {
            lines.push(Line::styled(
                "No hardware tokens were detected. Ensure tokens are plugged in and rescan.",
                Style::new().fg(COLOR_ORANGE).italic(),
            ));
            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::styled("[R]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(
                    " to rescan for tokens  |  ",
                    Style::new().fg(COLOR_TEXT_DEFAULT),
                ),
                Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(
                    " to continue with no tokens",
                    Style::new().fg(COLOR_TEXT_DEFAULT),
                ),
            ]));
        } else {
            // Show tokens
            lines.push(Line::styled(
                "Select token to use from detected hardware tokens:",
                Style::new().fg(COLOR_BORDER).bold(),
            ));
            lines.push(Line::default());
            for (index, card) in state.tokens.tokens.iter().enumerate() {
                let mut lock = card.try_lock().unwrap();
                let mut open_card = match lock.transaction() {
                    Ok(card) => card,
                    Err(_) => {
                        continue;
                    }
                };
                let app_identifier = open_card
                    .application_identifier()
                    .expect("Couldn't get card app_identifier");
                if index == self.selected {
                    // Highlight selected
                    lines.push(Line::from(vec![
                        Span::styled("[✓] Card: ", Style::new().fg(COLOR_SUCCESS).bold()),
                        Span::styled(app_identifier.ident(), Style::new().fg(COLOR_SOFT_PURPLE)),
                        Span::styled(" Manufacturer: ", Style::new().fg(COLOR_SUCCESS).bold()),
                        Span::styled(
                            app_identifier.manufacturer_name(),
                            Style::new().fg(COLOR_SOFT_PURPLE),
                        ),
                        Span::styled(" CardHolder Name: ", Style::new().fg(COLOR_SUCCESS).bold()),
                        Span::styled(
                            open_card.cardholder_name().unwrap_or("NOT SET".to_string()),
                            Style::new().fg(COLOR_SOFT_PURPLE),
                        ),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("[ ] Card: ", Style::new().fg(COLOR_TEXT_DEFAULT).bold()),
                        Span::styled(app_identifier.ident(), Style::new().fg(COLOR_SOFT_PURPLE)),
                        Span::styled(
                            " Manufacturer: ",
                            Style::new().fg(COLOR_TEXT_DEFAULT).bold(),
                        ),
                        Span::styled(
                            app_identifier.manufacturer_name(),
                            Style::new().fg(COLOR_SOFT_PURPLE),
                        ),
                        Span::styled(
                            " CardHolder Name: ",
                            Style::new().fg(COLOR_TEXT_DEFAULT).bold(),
                        ),
                        Span::styled(
                            open_card.cardholder_name().unwrap_or("NOT SET".to_string()),
                            Style::new().fg(COLOR_SOFT_PURPLE),
                        ),
                    ]));
                }
            }
            if self.selected >= state.tokens.tokens.len() {
                lines.push(Line::styled(
                    "[✓] Do not use a hardware token",
                    Style::new().fg(COLOR_SUCCESS).bold(),
                ));
            } else {
                lines.push(Line::styled(
                    "[ ] Do not use a hardware token",
                    Style::new().fg(COLOR_TEXT_DEFAULT),
                ));
            }

            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::styled("[R]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(
                    " to rescan for tokens  |  ",
                    Style::new().fg(COLOR_TEXT_DEFAULT),
                ),
                Span::styled("[TAB]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(" to select  |  ", Style::new().fg(COLOR_TEXT_DEFAULT)),
                Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(
                    " to continue with selected",
                    Style::new().fg(COLOR_TEXT_DEFAULT),
                ),
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
