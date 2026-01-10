use crate::state_handler::{actions::Action, setup_sequence::SetupState, state::State};
use crossterm::event::KeyEvent;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

pub trait Component {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
    where
        Self: Sized;
    fn move_with_state(self, state: &State) -> Self
    where
        Self: Sized;

    fn handle_key_event(&mut self, key: KeyEvent);
}

pub trait ComponentRender<Props> {
    fn render(&self, frame: &mut Frame, props: Props);
}

/// Used by SetupFlow pages to render and handle key events
pub trait SetupFlowRender {
    fn handle_key_event(
        &self,
        state: &SetupState,
        action_tx: &mut UnboundedSender<Action>,
        key: KeyEvent,
    );
    fn render(&self, state: &SetupState, frame: &mut Frame);
}
