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
// MediatorAsk
// ****************************************************************************
#[derive(Copy, Clone, Debug, Default)]
pub enum MediatorAsk {
    #[default]
    Default,
    Custom,
}
impl MediatorAsk {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            MediatorAsk::Default => MediatorAsk::Custom,
            MediatorAsk::Custom => MediatorAsk::Default,
        }
    }
}

impl MediatorAsk {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                state.mediator_ask = state.mediator_ask.switch();
            }
            KeyCode::Enter => {
                // User has chosen whether to create or import their BIP32 phrase
                state.props.state.active_page = match state.mediator_ask {
                    MediatorAsk::Default => SetupPage::UserName,
                    MediatorAsk::Custom => SetupPage::FinalPage,
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
            .title(" Choose Messaging Mediator ");

        let mut lines = vec![
            Line::styled(
                "Choose the DIDComm Messaging mediator:",
                Style::new().fg(COLOR_BORDER).bold(),
            ),
            Line::default(),
            Line::styled(
                "All communication occurs using secure messaging based on the DIDComm protocol.",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::styled(
                "The messaging service uses a mediator/relay, in some situations you may need to use a custom mediator DID.",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::default(),
        ];

        // Render the active chocie
        if let MediatorAsk::Default = self {
            lines.push(Line::styled(
                "[✓] Use Default LKMV Community Mediator (recommended)",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
            lines.push(Line::styled(
                "[ ] Use Custom Mediator (requires a mediator DID)",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
        } else {
            lines.push(Line::styled(
                "[ ] Use Default LKMV Community Mediator (recommended)",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
            lines.push(Line::styled(
                "[✓] Use Custom Mediator (requires a mediator DID)",
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
