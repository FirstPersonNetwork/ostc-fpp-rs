use crate::{state_handler::StateHandler, ui::UiManager};
use ratatui::style::Color;
#[cfg(unix)]
use tokio::signal::unix::signal;
use tokio::sync::broadcast;

mod state_handler;
mod ui;

// CLI Color codes
/// Success state - Completed actions, valid inputs, positive feedback
pub const COLOR_SUCCESS: Color = Color::Rgb(61, 220, 132); // #3DDC84 - Android Green

///Using bright blue for professional, accessible appearance
pub const COLOR_BORDER: Color = Color::Rgb(97, 175, 239); // #61AFEF - Blue

/// Warning state - Warnings, cautions, important notices
pub const COLOR_WARNING: Color = Color::Rgb(255, 184, 108); // #FFB86C - Orange

/// Warning state - Accessible red for important warnings and cautions
pub const COLOR_WARNING_ACCESSIBLE_RED: Color = Color::Rgb(220, 100, 100); // #DC6464 - Accessible Red

/// Default text color
pub const COLOR_TEXT_DEFAULT: Color = Color::White;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup the initial state
    let (terminator, mut interrupt_rx) = create_termination();
    let (state, state_rx) = StateHandler::new();
    let (ui_manager, action_rx) = UiManager::new();

    tokio::try_join!(
        state.main_loop(terminator, action_rx, interrupt_rx.resubscribe()),
        ui_manager.main_loop(state_rx, interrupt_rx.resubscribe()),
    )?;

    match interrupt_rx.recv().await {
        Ok(reason) => match reason {
            Interrupted::UserInt => println!("exited per user request"),
            Interrupted::OsSigInt => println!("exited because of an os sig int"),
            Interrupted::SystemError => println!("exited because of a system error"),
        },
        _ => {
            println!("exited because of an unexpected error");
        }
    }

    Ok(())
}

// ****************************************************************************
// Termination Management
// ****************************************************************************

#[derive(Debug, Clone)]
pub enum Interrupted {
    OsSigInt,
    UserInt,
    SystemError,
}

#[derive(Debug, Clone)]
pub struct Terminator {
    interrupt_tx: broadcast::Sender<Interrupted>,
}

impl Terminator {
    pub fn new(interrupt_tx: broadcast::Sender<Interrupted>) -> Self {
        Self { interrupt_tx }
    }

    pub fn terminate(&mut self, interrupted: Interrupted) -> anyhow::Result<()> {
        self.interrupt_tx.send(interrupted)?;

        Ok(())
    }
}

#[cfg(unix)]
async fn terminate_by_unix_signal(mut terminator: Terminator) {
    let mut interrupt_signal = signal(tokio::signal::unix::SignalKind::interrupt())
        .expect("failed to create interrupt signal stream");

    interrupt_signal.recv().await;

    terminator
        .terminate(Interrupted::OsSigInt)
        .expect("failed to send interrupt signal");
}

// create a broadcast channel for retrieving the application kill signal
pub fn create_termination() -> (Terminator, broadcast::Receiver<Interrupted>) {
    let (tx, rx) = broadcast::channel(1);
    let terminator = Terminator::new(tx);

    #[cfg(unix)]
    tokio::spawn(terminate_by_unix_signal(terminator.clone()));

    (terminator, rx)
}
