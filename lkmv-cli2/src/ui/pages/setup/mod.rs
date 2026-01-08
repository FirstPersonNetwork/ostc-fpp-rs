use crate::{
    state_handler::{actions::Action, state::State},
    ui::{
        component::{Component, ComponentRender},
        pages::Props,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

/// SetupPage handles the UI and the state for the initial setup interface
pub struct SetupPage {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,
    /// State Mapped SetupPage Props
    props: Props,
}

impl ComponentRender<()> for SetupPage {
    fn render(&self, frame: &mut Frame, _props: ()) {}
}

impl Component for SetupPage {
    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::F(10) => {
                let _ = self.action_tx.send(Action::Exit);
            }
            _ => {}
        }
    }

    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
    where
        Self: Sized,
    {
        SetupPage {
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
        SetupPage {
            props: Props::from(state),
            // propagate the update to the child components
            ..self
        }
    }
}
