use crate::{
    Interrupted,
    state_handler::{actions::Action, state::State},
};
use anyhow::Result;
use tokio::sync::{
    broadcast,
    mpsc::{self, UnboundedReceiver},
};

pub struct UiManager {
    action_tx: mpsc::UnboundedSender<Action>,
}

impl UiManager {
    pub fn new() -> (Self, UnboundedReceiver<Action>) {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        (Self { action_tx }, action_rx)
    }

    pub async fn main_loop(
        self,
        mut state_rx: UnboundedReceiver<State>,
        mut interrupt_rx: broadcast::Receiver<Interrupted>,
    ) -> Result<Interrupted> {
        Ok(Interrupted::UserInt)
    }
}
