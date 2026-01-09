use crossterm::event::KeyEvent;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    state_handler::{
        actions::Action,
        state::{ActivePage, State},
    },
    ui::{
        component::{Component, ComponentRender},
        pages::{main::MainPage, setup::choice::SetupChoicePage},
    },
};

pub mod main;
pub mod setup;

struct Props {
    active_page: ActivePage,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        Props {
            active_page: state.active_page,
        }
    }
}

pub struct AppRouter {
    props: Props,
    //
    main_page: MainPage,
    setup_choice_page: SetupChoicePage,
}

impl AppRouter {
    fn get_active_page_component_mut(&mut self) -> &mut dyn Component {
        match self.props.active_page {
            ActivePage::Main => &mut self.main_page,
            ActivePage::SetupChoice => &mut self.setup_choice_page,
        }
    }
}

impl Component for AppRouter {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
    where
        Self: Sized,
    {
        AppRouter {
            props: Props::from(state),
            //
            main_page: MainPage::new(state, action_tx.clone()),
            setup_choice_page: SetupChoicePage::new(state, action_tx.clone()),
        }
        .move_with_state(state)
    }

    fn move_with_state(self, state: &State) -> Self
    where
        Self: Sized,
    {
        AppRouter {
            props: Props::from(state),
            //
            main_page: self.main_page.move_with_state(state),
            setup_choice_page: self.setup_choice_page.move_with_state(state),
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        self.get_active_page_component_mut().handle_key_event(key)
    }
}

impl ComponentRender<()> for AppRouter {
    fn render(&self, frame: &mut Frame, props: ()) {
        match self.props.active_page {
            ActivePage::Main => self.main_page.render(frame, props),
            ActivePage::SetupChoice => self.setup_choice_page.render(frame, props),
        }
    }
}
