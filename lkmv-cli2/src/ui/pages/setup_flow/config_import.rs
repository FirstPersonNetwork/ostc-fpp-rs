use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{COLOR_BORDER, COLOR_TEXT_DEFAULT, COLOR_WARNING_ACCESSIBLE_RED};
use ratatui::{
    Frame,
    layout::{
        Constraint::{Length, Min},
        Layout,
    },
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    state_handler::{
        actions::Action,
        setup_sequence::{ConfigImport, SetupState},
    },
    ui::{component::SetupFlowRender, pages::setup_flow::render_setup_header},
};

impl SetupFlowRender for ConfigImport {
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
            _ => {}
        }
    }

    fn render(&self, state: &SetupState, frame: &mut Frame) {
        let [top, middle, bottom] =
            Layout::vertical([Length(3), Min(0), Length(3)]).areas(frame.area());

        render_setup_header(frame, top, state);

        frame.render_widget(
            Paragraph::new(Line::styled(
                "NOT IMPLEMENTED YET",
                Style::new().fg(COLOR_WARNING_ACCESSIBLE_RED).bold(),
            ))
            .block(
                Block::bordered()
                    .fg(COLOR_WARNING_ACCESSIBLE_RED)
                    .padding(Padding::proportional(1)),
            ),
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
