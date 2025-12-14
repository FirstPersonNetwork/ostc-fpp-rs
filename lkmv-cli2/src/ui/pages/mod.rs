use crossterm::event::KeyEvent;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    state_handler::{actions::Action, state::State},
    ui::{
        component::{Component, ComponentRender},
        pages::{main::MainPage, setup::SetupPage},
    },
};

pub mod main;
pub mod setup;

enum ActivePage {
    MainPage,
    SetupPage,
}

struct Props {
    active_page: ActivePage,
}

impl From<&State> for Props {
    fn from(_: &State) -> Self {
        Props {
            active_page: ActivePage::MainPage,
        }
    }
}

pub struct AppRouter {
    props: Props,
    //
    main_page: MainPage,
    setup_page: SetupPage,
}

impl AppRouter {
    fn get_active_page_component_mut(&mut self) -> &mut dyn Component {
        match self.props.active_page {
            ActivePage::MainPage => &mut self.main_page,
            ActivePage::SetupPage => &mut self.setup_page,
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
            setup_page: SetupPage::new(state, action_tx.clone()),
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
            setup_page: self.setup_page.move_with_state(state),
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        self.get_active_page_component_mut().handle_key_event(key)
    }
}

impl ComponentRender<()> for AppRouter {
    fn render(&self, frame: &mut Frame, props: ()) {
        match self.props.active_page {
            ActivePage::MainPage => self.main_page.render(frame, props),
            ActivePage::SetupPage => self.main_page.render(frame, props),
        }
    }
}
