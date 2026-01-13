use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{COLOR_BORDER, COLOR_SUCCESS, COLOR_TEXT_DEFAULT};
use ratatui::{
    Frame,
    layout::{
        Alignment,
        Constraint::{Length, Min, Percentage},
        Layout, Margin, Rect,
    },
    style::{Style, Stylize},
    symbols::merge::MergeStrategy,
    text::{Line, Span},
    widgets::{Block, BorderType, Padding, Paragraph},
};

use crate::{
    state_handler::{
        actions::Action,
        setup_sequence::{SetupPage, SetupState},
    },
    ui::pages::setup_flow::{SetupFlow, render_setup_header},
};

// ****************************************************************************
// StartAsk
// ****************************************************************************
#[derive(Copy, Clone, Debug, Default)]
pub enum StartAskPanel {
    #[default]
    Create,
    Import,
}

impl StartAskPanel {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            StartAskPanel::Create => StartAskPanel::Import,
            StartAskPanel::Import => StartAskPanel::Create,
        }
    }
}

impl StartAskPanel {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                // Switch active panel
                state.start_ask = state.start_ask.switch();
            }
            KeyCode::Enter => match state.start_ask {
                StartAskPanel::Create => {
                    state.props.state.active_page = SetupPage::BIP32PhraseAsk;
                }
                StartAskPanel::Import => {
                    state.props.state.active_page = SetupPage::ConfigImport;
                }
            },
            _ => {}
        }
    }

    pub fn render(&self, state: &SetupState, frame: &mut Frame) {
        let [top, middle, bottom] =
            Layout::vertical([Length(3), Min(0), Length(3)]).areas(frame.area());

        render_setup_header(frame, top, state);

        // Render the middle selection boxes
        let middle = Layout::horizontal([Percentage(50), Percentage(50)]).split(middle);
        let middle_left = middle[0].inner(Margin::new(2, 0));
        let middle_right = middle[1].inner(Margin::new(2, 0));

        render_left_panel(frame, middle_left, self);
        render_right_panel(frame, middle_right, self);

        let bottom_line = Line::from(vec![
            Span::styled("[TAB]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to navigate  |  ", Style::new().fg(COLOR_TEXT_DEFAULT)),
            Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to select  |  ", Style::new().fg(COLOR_TEXT_DEFAULT)),
            Span::styled("[F10]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to quit", Style::new().fg(COLOR_TEXT_DEFAULT)),
        ]);

        frame.render_widget(
            Paragraph::new(bottom_line).block(Block::new().padding(Padding::new(2, 0, 1, 0))),
            bottom,
        );
    }
}

// ****************************************************************************
// Render Left Panel (Setup new profile)
// ****************************************************************************
fn render_left_panel(frame: &mut Frame, rect: Rect, state: &StartAskPanel) {
    let block = if let StartAskPanel::Create = state {
        Block::bordered()
            .merge_borders(MergeStrategy::Fuzzy)
            .border_type(BorderType::Double)
            .fg(COLOR_SUCCESS)
            .padding(Padding::proportional(1))
            .title(" Setup new profile ")
    } else {
        Block::bordered()
            .merge_borders(MergeStrategy::Fuzzy)
            .fg(COLOR_BORDER)
            .padding(Padding::proportional(1))
            .title(" Setup new profile ")
    };

    let mut lines = vec![
        Line::styled(
            "Create and configure a brand new profile from scratch.",
            Style::new().fg(COLOR_TEXT_DEFAULT),
        ),
        Line::default(),
        Line::styled("You will:", Style::new().fg(COLOR_TEXT_DEFAULT)),
        Line::styled(
            "• Set up key management",
            Style::new().fg(COLOR_TEXT_DEFAULT),
        ),
        Line::styled("• Choose mediator", Style::new().fg(COLOR_TEXT_DEFAULT)),
        Line::styled(
            "• Create your Decentralized Identifier (DID)",
            Style::new().fg(COLOR_TEXT_DEFAULT),
        ),
        Line::styled("• Verify setup", Style::new().fg(COLOR_TEXT_DEFAULT)),
    ];

    if let StartAskPanel::Create = state {
        lines.push(Line::default());
        lines.push(Line::styled(
            "▶ Selected",
            Style::new().fg(COLOR_SUCCESS).bold(),
        ));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .block(block),
        rect,
    );
}

// ****************************************************************************
// Render Right Panel (Recovery from backup)
// ****************************************************************************
fn render_right_panel(frame: &mut Frame, rect: Rect, state: &StartAskPanel) {
    let block = if let StartAskPanel::Import = state {
        Block::bordered()
            .merge_borders(MergeStrategy::Fuzzy)
            .border_type(BorderType::Double)
            .fg(COLOR_SUCCESS)
            .padding(Padding::proportional(1))
            .title(" Recovery from backup ")
    } else {
        Block::bordered()
            .merge_borders(MergeStrategy::Fuzzy)
            .fg(COLOR_BORDER)
            .padding(Padding::proportional(1))
            .title(" Recovery from backup ")
    };

    let mut lines = vec![
        Line::styled(
            "Restore from an existing .lkmv backup file.",
            Style::new().fg(COLOR_TEXT_DEFAULT),
        ),
        Line::default(),
        Line::styled("Requires:", Style::new().fg(COLOR_TEXT_DEFAULT)),
        Line::styled("• Path to .lkmv file", Style::new().fg(COLOR_TEXT_DEFAULT)),
        Line::styled(
            "• Unlock code (if set)",
            Style::new().fg(COLOR_TEXT_DEFAULT),
        ),
        Line::styled("• Verification", Style::new().fg(COLOR_TEXT_DEFAULT)),
    ];

    if let StartAskPanel::Import = state {
        lines.push(Line::default());
        lines.push(Line::styled(
            "▶ Selected",
            Style::new().fg(COLOR_SUCCESS).bold(),
        ));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .block(block),
        rect,
    );
}
