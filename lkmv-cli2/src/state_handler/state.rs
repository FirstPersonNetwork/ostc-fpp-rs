use std::fmt::Display;

use crossterm::event::KeyCode;
use ratatui::Frame;

/// State holds the state of the application
#[derive(Default, Debug, Clone)]
pub struct State {
    pub main_menu: MainMenu,
}

#[derive(Default, Debug, Clone)]
pub enum MainMenu {
    #[default]
    Relationships,
    Tasks,
    Credentials,
}

impl Display for MainMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MainMenu::Relationships => write!(f, "Relationships"),
            MainMenu::Tasks => write!(f, "Tasks"),
            MainMenu::Credentials => write!(f, "Credentials"),
        }
    }
}

impl MainMenu {
    /// Returns the previous MainMenu item
    pub fn prev(&self) -> MainMenu {
        match self {
            MainMenu::Relationships => MainMenu::Credentials,
            MainMenu::Tasks => MainMenu::Relationships,
            MainMenu::Credentials => MainMenu::Tasks,
        }
    }

    /// Returns the next MainMenu item
    pub fn next(&self) -> MainMenu {
        match self {
            MainMenu::Relationships => MainMenu::Tasks,
            MainMenu::Tasks => MainMenu::Credentials,
            MainMenu::Credentials => MainMenu::Relationships,
        }
    }
}
