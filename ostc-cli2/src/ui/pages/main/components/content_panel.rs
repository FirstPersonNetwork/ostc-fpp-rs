use crate::state_handler::main_page::{
    content::ContentPanelState,
    menu::{MainMenu, MenuPanelState},
};
use ostc::colors::{
    COLOR_BORDER, COLOR_ORANGE, COLOR_SUCCESS, COLOR_TEXT_DEFAULT, COLOR_WARNING_ACCESSIBLE_RED,
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Stylize,
    symbols::merge::MergeStrategy,
    text::Line,
    widgets::{Block, BorderType, Paragraph},
};

// ****************************************************************************
// Render the Content panel
// ****************************************************************************
impl ContentPanelState {
    /// Render the content panel based on current state
    pub fn render(&self, frame: &mut Frame, rect: Rect, menu: &MenuPanelState) {
        // The surrounding block for the menu

        let content_block = if self.selected {
            Block::bordered()
                .merge_borders(MergeStrategy::Fuzzy)
                .border_type(BorderType::Double)
                .fg(COLOR_SUCCESS)
                .title("Content")
        } else {
            Block::bordered()
                .merge_borders(MergeStrategy::Fuzzy)
                .fg(COLOR_BORDER)
                .title("Content")
        };

        let lines = match menu.selected_menu {
            MainMenu::Settings => {
                vec![
                    Line::from(""),
                    Line::from("Managing settings has not been implemented yet").fg(COLOR_ORANGE),
                    Line::from("Press Enter to select a menu item").fg(COLOR_ORANGE),
                ]
            }
            MainMenu::Help => {
                vec![
                    Line::from(""),
                    Line::from("Press Up/Down to navigate the menu").fg(COLOR_TEXT_DEFAULT),
                    Line::from("Press Enter to select a menu item").fg(COLOR_TEXT_DEFAULT),
                ]
            }
            MainMenu::Quit => {
                vec![
                    Line::from(""),
                    Line::from("Press <Enter> to quit the application")
                        .fg(COLOR_WARNING_ACCESSIBLE_RED),
                ]
            }
            _ => {
                vec![
                    Line::from("Where is my content?").dark_gray(),
                    Line::from(menu.selected_menu.to_string()).blue(),
                ]
            }
        };

        frame.render_widget(
            Paragraph::new(lines)
                .alignment(Alignment::Left)
                .block(content_block),
            rect,
        );
    }
}
