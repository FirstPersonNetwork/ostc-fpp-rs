use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Stylize,
    symbols::merge::MergeStrategy,
    widgets::{Block, Paragraph},
};

use crate::{COLOR_BORDER, COLOR_SUCCESS, state_handler::main_page::content::ContentPanelState};

// ****************************************************************************
// Render the Content panel
// ****************************************************************************
impl ContentPanelState {
    /// Render the content panel based on current state
    pub fn render(&self, frame: &mut Frame, rect: Rect) {
        // The surrounding block for the menu

        let menu_block = if self.selected {
            Block::bordered()
                .merge_borders(MergeStrategy::Fuzzy)
                .fg(COLOR_SUCCESS)
                .title("Content")
        } else {
            Block::bordered()
                .merge_borders(MergeStrategy::Fuzzy)
                .fg(COLOR_BORDER)
                .title("Content")
        };

        frame.render_widget(
            Paragraph::new("Where is my content?")
                .dark_gray()
                .alignment(Alignment::Left)
                .block(menu_block),
            rect,
        );
    }
}
