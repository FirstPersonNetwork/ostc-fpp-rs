use crate::{
    BORDER_COLOR,
    state_handler::{
        actions::Action,
        state::{MainMenu, State},
    },
    ui::component::{Component, ComponentRender},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame,
    layout::{
        Alignment,
        Constraint::{Length, Min, Percentage},
        Layout, Rect,
    },
    style::Stylize,
    symbols::merge::MergeStrategy,
    widgets::{Block, Borders, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

/// MainPage handles the UI and the state of the primary lkmv interface
pub struct MainPage {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,

    /// State Mapped MainPage Props
    props: Props,
}

struct Props {
    main_menu: MainMenu,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        Props {
            main_menu: state.main_menu.clone(),
        }
    }
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
            KeyCode::Up => {
                // Handle Up key
                let _ = self
                    .action_tx
                    .send(Action::MainMenuSelected(self.props.main_menu.prev()));
            }
            KeyCode::Down => {
                // Handle Down key
                let _ = self
                    .action_tx
                    .send(Action::MainMenuSelected(self.props.main_menu.next()));
            }
            _ => {}
        }
    }
}

impl MainMenu {
    /// Render the main menu based on current state
    fn render(&self, frame: &mut Frame, rect: Rect) {
        let menu_block = Block::bordered()
            .merge_borders(MergeStrategy::Fuzzy)
            .fg(BORDER_COLOR);
        frame.render_widget(
            Paragraph::new(format!("Current Menu: {}", self))
                .dark_gray()
                .alignment(Alignment::Center)
                .block(menu_block),
            rect,
        );
    }
}

// ****************************************************************************
// Render the page
// ****************************************************************************
impl ComponentRender<()> for MainPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let [main_top, main_middle, main_bottom] =
            Layout::vertical([Length(2), Min(0), Length(2)]).areas(frame.area());

        let middle = Layout::horizontal([Percentage(33), Min(0)]).split(main_middle);

        let top_block = Block::new()
            .borders(Borders::BOTTOM)
            .merge_borders(MergeStrategy::Fuzzy)
            .fg(BORDER_COLOR);
        frame.render_widget(
            Paragraph::new("Title Area")
                .dark_gray()
                .alignment(Alignment::Center)
                .block(top_block),
            main_top,
        );

        // Middle block
        // Left = menu
        // right = actual content

        self.props.main_menu.render(frame, middle[0]);
        self.props.main_menu.render(frame, middle[1]);

        let bottom_block = Block::new()
            .borders(Borders::TOP)
            .merge_borders(MergeStrategy::Fuzzy)
            .fg(BORDER_COLOR);
        frame.render_widget(
            Paragraph::new("Bottom Block")
                .dark_gray()
                .alignment(Alignment::Center)
                .block(bottom_block),
            main_bottom,
        );
    }
}
