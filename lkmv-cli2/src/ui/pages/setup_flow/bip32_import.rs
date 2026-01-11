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
use tokio::sync::mpsc::UnboundedSender;
use tui_input::Input;

use crate::{
    state_handler::{
        actions::Action,
        setup_sequence::{BIP32PhraseImport, SetupState},
    },
    ui::{component::SetupFlowRender, pages::setup_flow::render_setup_header},
};

impl SetupFlowRender for BIP32PhraseImport {
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
            KeyCode::Enter => {
                let _ = action_tx.send(Action::SetupBIP32PhraseImportSubmit);
            }
            KeyCode::Esc => {
                let _ = action_tx.send(Action::SetupBIP32PhraseImportClear);
            }
            _ => {
                // Handle text input for mnemonic here
                let _ = action_tx.send(Action::SetupBIP32PhraseImportKey(Event::Key(key)));
            }
        }
    }

    fn render(&self, state: &SetupState, frame: &mut Frame<'_>) {
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
                Span::styled(" to clear input  |  ", Style::new().fg(COLOR_BORDER)),
                Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
                Span::styled(" to continue", Style::new().fg(COLOR_BORDER)),
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
