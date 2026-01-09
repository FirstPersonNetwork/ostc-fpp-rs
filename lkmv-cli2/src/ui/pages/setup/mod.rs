use lkmv::colors::{COLOR_BORDER, COLOR_DARK_GRAY, COLOR_ORANGE, COLOR_SUCCESS};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
};

use crate::state_handler::state::ActivePage;

pub mod bip32_initialization;
pub mod choice;
pub mod import_backup;

/// Renders the top headline for the setup pages
pub fn render_setup_header(frame: &mut Frame, rect: Rect, active_page: ActivePage) {
    let mut line1 = Line::default();
    let mut step = 0;

    if let ActivePage::SetupChoice = active_page {
        step = 1;
        line1.push_span(Span::styled(
            "● Choice",
            Style::new().fg(COLOR_ORANGE).bold(),
        ));
    } else {
        line1.push_span(Span::styled("✓ Choice", Style::new().fg(COLOR_SUCCESS)));
    }

    line1.push_span(Span::styled(
        " → ○ Key Management → ○ Mediator → ○ DID → ○ Verify ",
        Style::new().fg(COLOR_DARK_GRAY),
    ));

    let line2 = Line::from(Span::styled(
        format!("Section {}/5", step),
        Style::new().fg(COLOR_BORDER),
    ));

    frame.render_widget(
        Paragraph::new(vec![line1, line2])
            .alignment(Alignment::Left)
            .block(Block::new().padding(Padding::new(2, 0, 0, 0))),
        rect,
    );
}
