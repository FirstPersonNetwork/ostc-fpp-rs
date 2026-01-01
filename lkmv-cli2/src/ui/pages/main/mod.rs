use crate::{
    COLOR_BORDER, COLOR_SUCCESS, COLOR_TEXT_DEFAULT,
    state_handler::{
        actions::Action,
        main_page::{MainPageState, menu::MainMenu},
        state::{MainPanel, State},
    },
    ui::component::{Component, ComponentRender},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame,
    layout::{
        Alignment,
        Constraint::{Length, Min, Percentage},
        Layout,
    },
    style::Stylize,
    symbols::merge::MergeStrategy,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

pub mod components;

/// MainPage handles the UI and the state of the primary lkmv interface
pub struct MainPage {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,

    /// State Mapped MainPage Props
    props: Props,
}

struct Props {
    main_page: MainPageState,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        Props {
            main_page: state.main_page.clone(),
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
                if self.props.main_page.menu_panel.selected {
                    let _ = self.action_tx.send(Action::MainMenuSelected(
                        self.props.main_page.menu_panel.selected_menu.prev(),
                    ));
                }
            }
            KeyCode::Down => {
                // Handle Down key
                if self.props.main_page.menu_panel.selected {
                    let _ = self.action_tx.send(Action::MainMenuSelected(
                        self.props.main_page.menu_panel.selected_menu.next(),
                    ));
                }
            }
            KeyCode::Tab => {
                // Switch active panel
                let next_panel = match self.props.main_page.menu_panel.selected {
                    true => MainPanel::ContentPanel,
                    false => MainPanel::MainMenu,
                };
                let _ = self.action_tx.send(Action::MainPanelSwitch(next_panel));
            }
            KeyCode::Enter => {
                // Handle Enter key
                if self.props.main_page.menu_panel.selected_menu == MainMenu::Quit {
                    // Stop the application with a termination action
                    let _ = self.action_tx.send(Action::Exit);
                } else if self.props.main_page.menu_panel.selected {
                    // Switch to the content panel
                    let _ = self
                        .action_tx
                        .send(Action::MainPanelSwitch(MainPanel::ContentPanel));
                }
            }
            _ => {}
        }
    }
}

// ****************************************************************************
// Render the page
// ****************************************************************************
impl ComponentRender<()> for MainPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let [main_top, main_middle, main_bottom] =
            Layout::vertical([Length(2), Min(0), Length(3)]).areas(frame.area());

        let top = Layout::horizontal([Percentage(50), Percentage(50)]).split(main_top);
        let middle = Layout::horizontal([Percentage(20), Min(0)]).split(main_middle);

        frame.render_widget(
            Paragraph::new(" LKMV Dashboard")
                .fg(COLOR_SUCCESS)
                .alignment(Alignment::Left),
            top[0],
        );
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("Glenn Gore ").fg(COLOR_SUCCESS),
                Line::from("🆔 did:webvh:scid ").fg(COLOR_TEXT_DEFAULT),
            ])
            .alignment(Alignment::Right),
            top[1],
        );

        // Middle block
        // Left = menu
        // right = actual content

        // Main Menu
        self.props.main_page.menu_panel.render(frame, middle[0]);
        self.props.main_page.content_panel.render(
            frame,
            middle[1],
            &self.props.main_page.menu_panel,
        );

        let bottom_block = Block::new()
            .borders(Borders::TOP)
            .merge_borders(MergeStrategy::Fuzzy)
            .fg(COLOR_BORDER);
        frame.render_widget(
            Paragraph::new("<TAB> to change panels")
                .dark_gray()
                .alignment(Alignment::Left)
                .block(bottom_block),
            main_bottom,
        );
    }
}
