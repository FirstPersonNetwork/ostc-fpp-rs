use std::sync::Arc;

use crossterm::event::{Event, KeyCode, KeyEvent};
use lkmv::colors::{
    COLOR_BORDER, COLOR_DARK_GRAY, COLOR_ORANGE, COLOR_SOFT_PURPLE, COLOR_SUCCESS,
    COLOR_TEXT_DEFAULT, COLOR_WARNING_ACCESSIBLE_RED,
};
use openpgp_card::{Card, state::Open};
use ratatui::{
    Frame,
    layout::{
        Constraint::{Length, Min},
        Layout, Margin, Rect,
    },
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph, Wrap},
};
use secrecy::SecretString;
use tokio::sync::Mutex;
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    state_handler::{
        actions::Action,
        setup_sequence::{SetupPage, SetupState},
    },
    ui::pages::setup_flow::{SetupFlow, render_setup_header},
};

#[derive(Clone, Default)]
pub struct TokenSelect {
    pub selected: usize,
    pub selected_token: Option<Arc<Mutex<Card<Open>>>>,
    pub ask_admin_pin: bool,
    pub token_admin_pin: Input,
}

impl TokenSelect {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Down if !state.token_select.ask_admin_pin => {
                let token_count = state.props.state.tokens.tokens.len();
                state.token_select.selected = (state.token_select.selected + 1) % (token_count + 1);
            }
            KeyCode::Up if !state.token_select.ask_admin_pin => {
                let token_count = state.props.state.tokens.tokens.len();
                if state.token_select.selected == 0 {
                    state.token_select.selected = token_count;
                } else {
                    state.token_select.selected -= 1;
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') if !state.token_select.ask_admin_pin => {
                let _ = state.action_tx.send(Action::GetTokens);
            }
            KeyCode::Esc if state.token_select.ask_admin_pin => {
                state.token_select.token_admin_pin.reset();
            }
            KeyCode::Enter => {
                if state.token_select.ask_admin_pin {
                    // Get Admin PIN from input
                    let admin_pin = if state.token_select.token_admin_pin.value().is_empty() {
                        SecretString::new("12345678".to_string())
                    } else {
                        SecretString::new(state.token_select.token_admin_pin.value().to_string())
                    };
                    let _ = state.action_tx.send(Action::SetAdminPin(admin_pin));
                } else {
                    // Selected Token - Now get Admin PIN
                    if state.token_select.selected == state.props.state.tokens.tokens.len() {
                        // No token selected
                        state.token_select.selected_token = None;
                        state.props.state.active_page = SetupPage::UnlockCodeAsk;
                    } else {
                        state.token_select.selected_token = Some(
                            state.props.state.tokens.tokens[state.token_select.selected].clone(),
                        );
                        state.token_select.ask_admin_pin = true;
                    }
                }
            }
            _ if state.token_select.ask_admin_pin => {
                state
                    .token_select
                    .token_admin_pin
                    .handle_event(&Event::Key(key));
            }
            _ => {}
        }
    }

    pub fn render(&self, state: &SetupState, frame: &mut Frame) {
        let [top, middle, bottom] =
            Layout::vertical([Length(3), Min(0), Length(3)]).areas(frame.area());

        render_setup_header(frame, top, state);

        if self.ask_admin_pin {
            // Need to get ADMIN Pin from the user
            let mut lock = state.tokens.tokens[self.selected].try_lock().unwrap();
            let mut open_card = match lock.transaction() {
                Ok(card) => card,
                Err(e) => {
                    panic!(
                        "Selected a token but then couldn't read from it - likely could have been unplugged. Reason: {e}"
                    );
                }
            };
            let app_identifier = open_card
                .application_identifier()
                .expect("Couldn't get card app_identifier");

            // 0: Selected Token
            // 1: Input 0 Header (ADMIN PIN)
            // 2: INPUT <-- ADMIN PIN
            // 3: Key Bindings
            let content: [Rect; 4] = Layout::vertical([Length(2), Length(2), Length(2), Min(0)])
                .areas(middle.inner(Margin::new(3, 2)));

            let [input0_prompt, input0_box] =
                Layout::horizontal([Length(11), Min(0)]).areas(content[2]);

            let block = Block::bordered()
                .fg(COLOR_BORDER)
                .padding(Padding::proportional(1))
                .title(" Step 2/5: Get Admin PIN ");
            frame.render_widget(block, middle);

            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Selected Card: ", Style::new().fg(COLOR_SUCCESS).bold()),
                    Span::styled(
                        app_identifier.ident(),
                        Style::new().fg(COLOR_SOFT_PURPLE).bold(),
                    ),
                    Span::styled(" Manufacturer: ", Style::new().fg(COLOR_SUCCESS).bold()),
                    Span::styled(
                        app_identifier.manufacturer_name(),
                        Style::new().fg(COLOR_SOFT_PURPLE).bold(),
                    ),
                    Span::styled(" CardHolder Name: ", Style::new().fg(COLOR_SUCCESS).bold()),
                    Span::styled(
                        open_card.cardholder_name().unwrap_or("NOT SET".to_string()),
                        Style::new().fg(COLOR_SOFT_PURPLE).bold(),
                    ),
                ])),
                content[0],
            );

            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        "Enter the Admin PIN for the selected token:",
                        Style::new().fg(COLOR_BORDER).bold(),
                    ),
                    Span::styled(
                        " (leave blank for default PIN: ",
                        Style::new().fg(COLOR_DARK_GRAY).bold(),
                    ),
                    Span::styled("12345678", Style::new().fg(COLOR_DARK_GRAY)),
                    Span::styled(")", Style::new().fg(COLOR_DARK_GRAY).bold()),
                ])),
                content[1],
            );

            frame.render_widget(
                Paragraph::new(Span::styled(
                    "Admin Pin: ",
                    Style::new().fg(COLOR_SOFT_PURPLE).bold(),
                )),
                input0_prompt,
            );
            render_input(&self.token_admin_pin, frame, input0_box);

            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[ESC]", Style::new().fg(COLOR_BORDER).bold()),
                    Span::styled(" to clear input  |  ", Style::new().fg(COLOR_TEXT_DEFAULT)),
                    Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
                    Span::styled(" to continue", Style::new().fg(COLOR_TEXT_DEFAULT)),
                ])),
                content[3],
            );
        } else {
            let block = Block::bordered()
                .fg(COLOR_BORDER)
                .padding(Padding::proportional(1))
                .title(" Step 1/5: Select Hardware Token ");

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
                            Span::styled(
                                app_identifier.ident(),
                                Style::new().fg(COLOR_SOFT_PURPLE).bold(),
                            ),
                            Span::styled(" Manufacturer: ", Style::new().fg(COLOR_SUCCESS).bold()),
                            Span::styled(
                                app_identifier.manufacturer_name(),
                                Style::new().fg(COLOR_SOFT_PURPLE).bold(),
                            ),
                            Span::styled(
                                " CardHolder Name: ",
                                Style::new().fg(COLOR_SUCCESS).bold(),
                            ),
                            Span::styled(
                                open_card.cardholder_name().unwrap_or("NOT SET".to_string()),
                                Style::new().fg(COLOR_SOFT_PURPLE).bold(),
                            ),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled("[ ] Card: ", Style::new().fg(COLOR_TEXT_DEFAULT)),
                            Span::styled(
                                app_identifier.ident(),
                                Style::new().fg(COLOR_SOFT_PURPLE),
                            ),
                            Span::styled(" Manufacturer: ", Style::new().fg(COLOR_TEXT_DEFAULT)),
                            Span::styled(
                                app_identifier.manufacturer_name(),
                                Style::new().fg(COLOR_SOFT_PURPLE),
                            ),
                            Span::styled(" CardHolder Name: ", Style::new().fg(COLOR_TEXT_DEFAULT)),
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

                frame.render_widget(
                    Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
                    middle,
                );
            }
        }

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

fn render_input(input: &Input, frame: &mut Frame, area: Rect) {
    // keep 1 for borders and 1 for cursor
    let width = area.width.max(3) - 3;
    let scroll = input.visual_scroll(width as usize);
    let mut s = String::new();
    for _ in 0..input.value().len() {
        s.push('*');
    }
    let text = Span::styled(s, Style::new().fg(COLOR_SOFT_PURPLE));

    frame.render_widget(Paragraph::new(text).scroll((0, scroll as u16)), area);

    let x = input.visual_cursor().max(scroll) - scroll;
    frame.set_cursor_position((area.x + x as u16, area.y))
}
