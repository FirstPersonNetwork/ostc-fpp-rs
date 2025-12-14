use std::io::{self, Stdout};

use crate::{
    Interrupted,
    state_handler::{actions::Action, state::State},
    ui::{
        component::{Component, ComponentRender},
        pages::AppRouter,
    },
};
use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, Event, EventStream},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, prelude::CrosstermBackend};
use tokio::sync::{
    broadcast,
    mpsc::{self, UnboundedReceiver},
};
use tokio_stream::StreamExt;

pub mod component;
pub mod pages;

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
        let mut terminal = setup_terminal()?;

        let mut crossterm_events = EventStream::new();

        // consume the first state to initialize the ui app
        let mut app_router = {
            match state_rx.recv().await {
                Some(state) => AppRouter::new(&state, self.action_tx.clone()),
                _ => {
                    return Err(anyhow::anyhow!("could not get the initial state"));
                }
            }
        };

        let result: anyhow::Result<Interrupted> = loop {
            tokio::select! {
                // Catch and handle crossterm events
               maybe_event = crossterm_events.next() => match maybe_event {
                    Some(Ok(Event::Key(key)))  => {
                        app_router.handle_key_event(key);
                    },
                    None => break Ok(Interrupted::UserInt),
                    _ => (),
                },
                // Handle state updates
                Some(state) = state_rx.recv() => {
                    app_router = app_router.move_with_state(&state);
                },
                // Catch and handle interrupt signal to gracefully shutdown
                Ok(interrupted) = interrupt_rx.recv() => {
                    break Ok(interrupted);
                }
            }

            if let Err(err) = terminal
                .draw(|frame| app_router.render(frame, ()))
                .context("could not render to the terminal")
            {
                break Err(err);
            }
        };

        restore_terminal(&mut terminal)?;

        result
    }
}

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();

    enable_raw_mode()?;

    execute!(stdout, EnterAlternateScreen, DisableMouseCapture)?;

    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(terminal.show_cursor()?)
}
