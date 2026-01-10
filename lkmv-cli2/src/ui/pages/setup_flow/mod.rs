use crate::{
    state_handler::{
        actions::Action,
        setup_sequence::{SetupPage, SetupState},
        state::State,
    },
    ui::component::{Component, ComponentRender, SetupFlowRender},
};
use crossterm::event::{KeyEvent, KeyEventKind};
use lkmv::colors::{
    COLOR_BORDER, COLOR_DARK_GRAY, COLOR_ORANGE, COLOR_SUCCESS, COLOR_TEXT_DEFAULT,
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

pub mod bip32_ask;
pub mod bip32_show;
pub mod config_import;
pub mod start_ask;

/// Handles the Setup Flow sequence
pub struct SetupFlow {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,

    /// State Mapped MainPage Props
    props: Props,
}

struct Props {
    state: SetupState,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        Props {
            state: state.setup.clone(),
        }
    }
}

impl Component for SetupFlow {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
    where
        Self: Sized,
    {
        SetupFlow {
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
        SetupFlow {
            props: Props::from(state),
            // propagate the update to the child components
            ..self
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        self.props
            .state
            .active_page
            .handle_key_event(&self.props.state, &mut self.action_tx, key);
    }
}

// ****************************************************************************
// Render the page
// ****************************************************************************
impl ComponentRender<()> for SetupFlow {
    fn render(&self, frame: &mut Frame, _props: ()) {
        self.props
            .state
            .active_page
            .render(&self.props.state, frame);
    }
}

/// Renders the top headline for the setup pages
pub fn render_setup_header(frame: &mut Frame, rect: Rect, state: &SetupState) {
    let mut line1 = Line::default();
    let mut step = 0;

    if let SetupPage::StartAsk = state.active_page {
        step = 1;
        line1.push_span(Span::styled(
            "● Choice",
            Style::new().fg(COLOR_ORANGE).bold(),
        ));
    } else {
        line1.push_span(Span::styled("✓ Choice", Style::new().fg(COLOR_SUCCESS)));
    }

    if let SetupPage::BIP32PhraseAsk | SetupPage::BIP32PhraseShow = state.active_page {
        step = 2;
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled(
            "● Key Management",
            Style::new().fg(COLOR_ORANGE).bold(),
        ));
    } else if let SetupPage::ConfigImport = state.active_page {
        step = 2;
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled(
            "● Locate Backup",
            Style::new().fg(COLOR_ORANGE).bold(),
        ));
    }

    line1.push_span(Span::styled(
        " → ○ Mediator → ○ DID → ○ Verify ",
        Style::new().fg(COLOR_DARK_GRAY),
    ));

    let line2 = Line::from(Span::styled(
        format!("Section {}/5", step),
        Style::new().fg(COLOR_BORDER),
    ));

    frame.render_widget(
        Paragraph::new(vec![line1, line2])
            .alignment(Alignment::Left)
            .block(Block::new().padding(Padding::new(2, 0, 0, 0))),
        rect,
    );
}
