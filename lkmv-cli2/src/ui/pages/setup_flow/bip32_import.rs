use crossterm::event::{Event, KeyCode, KeyEvent};
use lkmv::colors::{COLOR_BORDER, COLOR_TEXT_DEFAULT};
use ratatui::{
    Frame,
    layout::{
        Constraint::{Length, Min},
        Layout, Margin, Rect,
    },
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    state_handler::{
        actions::Action,
        setup_sequence::{SetupPage, SetupState, bip32::BIP32_39},
    },
    ui::pages::setup_flow::{SetupFlow, render_setup_header},
};

// ****************************************************************************
// BIP32PhraseImport
// ****************************************************************************

#[derive(Clone, Debug, Default)]
pub struct BIP32PhraseImport {
    pub mnemonic: Input,
    pub warning_msg: Option<String>,
}

impl BIP32PhraseImport {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Enter => {
                // User has submitted their imported BIP32 phrase
                let input_phrase = state.bip32_import.mnemonic.value();

                // Validate the entered mnemonic
                match BIP32_39::from_mnemonic(input_phrase) {
                    Ok(bip32_39) => {
                        state.props.state.mnemonic = bip32_39;
                        // Proceed to the next setup step
                        state.props.state.active_page = SetupPage::DIDKeysAsk;
                    }
                    Err(e) => {
                        // Invalid mnemonic entered
                        state.bip32_import.warning_msg = Some(e.to_string());
                    }
                }
            }
            KeyCode::Esc => {
                state.bip32_import.mnemonic.reset();
            }
            _ => {
                // Handle text input for mnemonic here
                state.bip32_import.mnemonic.handle_event(&Event::Key(key));
            }
        }
    }

    pub fn render(&self, state: &SetupState, frame: &mut Frame<'_>) {
        let [top, middle, bottom] =
            Layout::vertical([Length(3), Min(0), Length(3)]).areas(frame.area());

        render_setup_header(frame, top, state);

        let content: [Rect; 4] = Layout::vertical([Length(2), Length(2), Length(2), Min(0)])
            .areas(middle.inner(Margin::new(3, 2)));

        let [input_prompt, input_box] = Layout::horizontal([Length(2), Min(0)]).areas(content[1]);

        frame.render_widget(
            Block::bordered()
                .fg(COLOR_BORDER)
                .padding(Padding::proportional(1))
                .title(" Step 2/4: Import BIP39 Recovery Phrase "),
            middle,
        );

        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(
                    "Enter your BIP39 mnemonic (24 words, separated by spaces):",
                    Style::new().fg(COLOR_TEXT_DEFAULT),
                ),
                Line::default(),
            ]),
            content[0],
        );
        frame.render_widget(
            Paragraph::new(Span::styled(">", Style::new().fg(COLOR_BORDER).bold())),
            input_prompt,
        );

        render_input(&self.mnemonic, frame, input_box);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("[ESC]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(" to clear input  |  ", Style::new().fg(COLOR_TEXT_DEFAULT)),
                Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(" to continue", Style::new().fg(COLOR_TEXT_DEFAULT)),
            ])),
            content[2],
        );

        if let Some(warning_msg) = &self.warning_msg {
            frame.render_widget(
                Paragraph::new(Line::styled(
                    warning_msg,
                    Style::new()
                        .fg(lkmv::colors::COLOR_WARNING_ACCESSIBLE_RED)
                        .bold(),
                )),
                content[3],
            );
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
    frame.render_widget(
        Paragraph::new(input.value())
            .fg(COLOR_TEXT_DEFAULT)
            .scroll((0, scroll as u16)),
        area,
    );

    let x = input.visual_cursor().max(scroll) - scroll;
    frame.set_cursor_position((area.x + x as u16, area.y))
}
