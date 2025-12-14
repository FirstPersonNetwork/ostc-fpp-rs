use crate::{
    Interrupted, Terminator,
    state_handler::{actions::Action, state::State},
};
use anyhow::Result;
use tokio::sync::{
    broadcast,
    mpsc::{self, UnboundedReceiver, UnboundedSender},
};

pub mod actions;
pub mod state;

pub struct StateHandler {
    state_tx: UnboundedSender<State>,
}

impl StateHandler {
    pub fn new() -> (Self, UnboundedReceiver<State>) {
        let (state_tx, state_rx) = mpsc::unbounded_channel::<State>();

        (StateHandler { state_tx }, state_rx)
    }

    pub async fn main_loop(
        self,
        mut terminator: Terminator,
        mut action_rx: UnboundedReceiver<Action>,
        mut interrupt_rx: broadcast::Receiver<Interrupted>,
    ) -> Result<Interrupted> {
        Ok(Interrupted::UserInt)
    }
}
