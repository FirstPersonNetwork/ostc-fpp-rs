use crate::{
    state_handler::{actions::Action, state::State},
    ui::{
        component::{Component, ComponentRender},
        pages::Props,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame,
    layout::{
        Constraint::{Fill, Length, Min},
        Layout, Margin,
    },
    style::Stylize,
    symbols::merge::MergeStrategy,
    widgets::Block,
};
use tokio::sync::mpsc::UnboundedSender;

/// MainPage handles the UI and the state of the primary lkmv interface
pub struct MainPage {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,
    /// State Mapped MainPage Props
    props: Props,
}

impl Component for MainPage {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
    where
        Self: Sized,
    {
        MainPage {
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
        MainPage {
            props: Props::from(state),
            // propagate the update to the child components
            ..self
        }
    }

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
}

impl ComponentRender<()> for MainPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let [main_top, main_middle, main_bottom] =
            Layout::vertical([Length(3), Min(0), Length(3)]).areas(frame.area());

        frame.render_widget(
            Block::bordered().merge_borders(MergeStrategy::Fuzzy),
            main_top,
        );
        frame.render_widget(
            Block::bordered().merge_borders(MergeStrategy::Fuzzy),
            main_middle.outer(Margin::new(1, 1)),
        );
        frame.render_widget(
            Block::bordered().merge_borders(MergeStrategy::Fuzzy),
            main_bottom,
        );
    }
}
