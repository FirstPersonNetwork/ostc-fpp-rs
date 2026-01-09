use crate::{
    state_handler::{
        actions::Action,
        setup_page::{ChoicePanel, ChoiceState, SetupPages},
        state::{ActivePage, State},
    },
    ui::{
        component::{Component, ComponentRender},
        pages::setup::render_setup_header,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
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
use tokio::sync::mpsc::UnboundedSender;

pub struct Props {
    active_page: ActivePage,
    choice_state: SetupPages,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        Props {
            active_page: state.active_page,
            choice_state: state.setup_page.active_page.clone(),
        }
    }
}

/// SetupChoicePage handles the UI and the state for the initial setup interface
pub struct SetupChoicePage {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,
    /// State Mapped SetupPage Props
    pub props: Props,
}

impl Component for SetupChoicePage {
    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::F(10) => {
                let _ = self.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                // Switch active panel
                let SetupPages::Choice(choice_state) = &self.props.choice_state;
                let _ = self.action_tx.send(Action::SetupChoicePanelSwitch(
                    choice_state.active_panel.switch(),
                ));
            }
            _ => {}
        }
    }

    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
    where
        Self: Sized,
    {
        SetupChoicePage {
            action_tx: action_tx.clone(),
            // set the props
            props: Props::from(state),
        }
        .move_with_state(state)
    }

    fn move_with_state(self, state: &State) -> Self
    where
        Self: Sized,
    {
        SetupChoicePage {
            props: Props::from(state),
            // propagate the update to the child components
            ..self
        }
    }
}

// ****************************************************************************
// Primary Render function for this page
// ****************************************************************************
impl ComponentRender<()> for SetupChoicePage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let SetupPages::Choice(state) = &self.props.choice_state;

        let [top, middle, bottom] =
            Layout::vertical([Length(3), Min(0), Length(3)]).areas(frame.area());

        render_setup_header(frame, top, self.props.active_page);

        // Render the middle selection boxes
        let middle = Layout::horizontal([Percentage(50), Percentage(50)]).split(middle);
        let middle_left = middle[0].inner(Margin::new(2, 0));
        let middle_right = middle[1].inner(Margin::new(2, 0));

        render_left_panel(frame, middle_left, state);
        render_right_panel(frame, middle_right, state);

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
fn render_left_panel(frame: &mut Frame, rect: Rect, state: &ChoiceState) {
    let block = if let ChoicePanel::Left = state.active_panel {
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

    if let ChoicePanel::Left = state.active_panel {
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
fn render_right_panel(frame: &mut Frame, rect: Rect, state: &ChoiceState) {
    let block = if let ChoicePanel::Right = state.active_panel {
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

    if let ChoicePanel::Right = state.active_panel {
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
