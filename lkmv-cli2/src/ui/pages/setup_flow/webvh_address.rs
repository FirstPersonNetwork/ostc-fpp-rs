use crossterm::event::{Event, KeyCode, KeyEvent};
use lkmv::colors::{COLOR_BORDER, COLOR_DARK_GRAY, COLOR_ORANGE, COLOR_SOFT_PURPLE, COLOR_TEXT_DEFAULT};
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
    state_handler::{actions::Action, setup_sequence::SetupState},
    ui::pages::setup_flow::{SetupFlow, render_setup_header},
};

// ****************************************************************************
// WebvhAddress
// ****************************************************************************

#[derive(Clone, Debug, Default)]
pub struct WebvhAddress {
    pub address: Input,
}

impl WebvhAddress {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Enter => {
                let _ = state.action_tx.send(Action::SetupCompleted(
                    state.webvh_address.address.value().to_string(),
                ));
            }
            KeyCode::Esc => {
                state.webvh_address.address.reset();
            }
            _ => {
                // Handle text input
                state.webvh_address.address.handle_event(&Event::Key(key));
            }
        }
    }

    pub fn render(&self, state: &SetupState, frame: &mut Frame<'_>) {
        let [top, middle, bottom] =
            Layout::vertical([Length(3), Min(0), Length(3)]).areas(frame.area());

        render_setup_header(frame, top, state);

        // 0: Input 0 Header
        // 1: INPUT
        // 2: Key Bindings
        let content: [Rect; 3] =
            Layout::vertical([Length(4), Length(2), Min(0)]).areas(middle.inner(Margin::new(3, 2)));

        let [input0_prompt, input0_box] = Layout::horizontal([Length(2), Min(0)]).areas(content[1]);

        frame.render_widget(
            Block::bordered()
                .fg(COLOR_BORDER)
                .padding(Padding::proportional(1))
                .title(" Step 2/2: Set up community DID "),
            middle,
        );

        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(
                    "Your identity within LKMV is represented using the Web Verifiable History (WebVH) DID method.", 
                    Style::new().fg(COLOR_DARK_GRAY)
                ),
                Line::default(),
                Line::styled(
                    "Enter the web address where your DID will be hosted:",
                    Style::new().fg(COLOR_BORDER).bold(),
                )
            ]),
            content[0],
        );

        frame.render_widget(
            Paragraph::new(Span::styled(
                "> ",
                Style::new().fg(COLOR_SOFT_PURPLE).bold(),
            )),
            input0_prompt,
        );

        render_input(&self.address, frame, input0_box);

        frame.render_widget(
            Paragraph::new(vec![
                Line::styled("ℹ️ Note: For example, if hosting your DID using GitHub Pages, use a URL like: ", Style::new().fg(COLOR_ORANGE)),
                Line::styled(
                    "         • https://<username>.github.io/",
                    Style::new().fg(COLOR_ORANGE).bold().italic(),
                ),
                Line::styled(
                    "         • https://<username>.github.io/lkmv-did/",
                    Style::new().fg(COLOR_ORANGE).bold().italic(),
                ),
                Line::default(),
                Line::from(vec![
                    Span::styled("[ESC]", Style::new().fg(COLOR_BORDER).bold()),
                    Span::styled(" to clear input  |  ", Style::new().fg(COLOR_TEXT_DEFAULT)),
                    Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
                    Span::styled(" to continue", Style::new().fg(COLOR_TEXT_DEFAULT)),
                ]),
                Line::default(),
                Line::styled("What is WebVH DID?", Style::new().fg(COLOR_BORDER).bold()),
                Line::styled("• Decentralized identifier accessible via HTTPS", Style::new().fg(COLOR_TEXT_DEFAULT)),
                Line::styled("• Changes are tracked using Verifiable History Logs", Style::new().fg(COLOR_TEXT_DEFAULT)),
                Line::styled("• No blockchain or external services required beyond simple web hosting", Style::new().fg(COLOR_TEXT_DEFAULT)),
                Line::styled("• Full control and ownership over your DID and where you choose to host it", Style::new().fg(COLOR_TEXT_DEFAULT)),
            ]),
            content[2],
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

fn render_input(input: &Input, frame: &mut Frame, area: Rect) {
    // keep 1 for borders and 1 for cursor
    let width = area.width.max(3) - 3;
    let scroll = input.visual_scroll(width as usize);

    frame.render_widget(
        Paragraph::new(Span::styled(
            input.value(),
            Style::new().fg(COLOR_SOFT_PURPLE),
        ))
        .scroll((0, scroll as u16)),
        area,
    );

    let x = input.visual_cursor().max(scroll) - scroll;
    frame.set_cursor_position((area.x + x as u16, area.y))
}
