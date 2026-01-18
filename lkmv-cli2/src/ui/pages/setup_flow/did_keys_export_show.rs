use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{
    COLOR_BORDER, COLOR_ORANGE, COLOR_SOFT_PURPLE, COLOR_SUCCESS, COLOR_TEXT_DEFAULT,
};
use ratatui::{
    Frame,
    layout::{
        Constraint::{Length, Min, Percentage},
        Layout, Margin,
    },
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph, Wrap},
};

use crate::{
    state_handler::{
        actions::Action,
        setup_sequence::{SetupPage, SetupState},
    },
    ui::pages::setup_flow::{SetupFlow, render_setup_header},
};

// ****************************************************************************
// DIDKeysExportShow
// ****************************************************************************
#[derive(Copy, Clone, Debug, Default)]
pub struct DIDKeysExportShow {
    clipboard_copy: bool,
}

impl DIDKeysExportShow {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                let mut clipboard = Clipboard::new().unwrap();
                clipboard
                    .set_text(state.props.state.did_keys_export.exported.clone().unwrap())
                    .unwrap();

                state.did_keys_export_show.clipboard_copy = true;
            }
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Enter => {
                #[cfg(feature = "openpgp-card")]
                {
                    state.props.state.active_page = SetupPage::TokenStart;
                }
                #[cfg(not(feature = "openpgp-card"))]
                {
                    state.props.state.active_page = SetupPage::UnlockCodeAsk;
                }
            }
            _ => {}
        }
    }

    pub fn render(&self, state: &SetupState, frame: &mut Frame) {
        let [top, middle, bottom] =
            Layout::vertical([Length(3), Min(0), Length(3)]).areas(frame.area());

        render_setup_header(frame, top, state);

        let block = Block::bordered()
            .fg(COLOR_BORDER)
            .padding(Padding::proportional(1))
            .title(" Step 4/4: Exported Private Keys ");

        frame.render_widget(block, middle);

        let [left, right] = Layout::horizontal([Percentage(30), Percentage(70)])
            .areas(middle.inner(Margin::new(3, 2)));

        let mut lines: Vec<Line> = vec![
            Line::styled(
                "Export Status",
                Style::new().fg(COLOR_BORDER).bold().underlined(),
            ),
            Line::default(),
        ];

        for msg in &state.did_keys_export.messages {
            lines.push(Line::styled(msg, Style::new().fg(COLOR_SUCCESS)));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), left);

        // Has the export completed? If so then display the exported armor text
        if let Some(exported) = &state.did_keys_export.exported {
            let mut lines = vec![
                Line::styled(
                    "Private Keys Successfully Exported",
                    Style::new().fg(COLOR_SUCCESS).bold(),
                ),
                Line::default(),
                Line::from(vec![
                    Span::styled("⚠ WARNING: ", Style::new().fg(COLOR_ORANGE).bold()),
                    Span::styled(
                        "Keep this key safe and secure!",
                        Style::new().fg(COLOR_BORDER),
                    ),
                ]),
                Line::default(),
            ];

            for line in exported.lines() {
                lines.push(Line::styled(line, Style::new().fg(COLOR_SOFT_PURPLE)));
            }

            lines.push(Line::default());
            if state.did_keys_export.exported.is_some() {
                lines.push(Line::from(vec![
                    Span::styled("[C]", Style::new().fg(COLOR_BORDER).bold()),
                    Span::styled(
                        " Copy to clipboard  |  ",
                        Style::new().fg(COLOR_TEXT_DEFAULT),
                    ),
                    Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
                    Span::styled(" to continue", Style::new().fg(COLOR_TEXT_DEFAULT)),
                ]));

                if self.clipboard_copy {
                    lines.push(Line::styled(
                        "✓ Copied to clipboard!",
                        Style::new().fg(COLOR_ORANGE).bold().slow_blink(),
                    ));
                }
            }
            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), right);
        }

        let bottom_line = Line::from(vec![
            Span::styled("[F10]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to quit", Style::new().fg(COLOR_TEXT_DEFAULT)),
        ]);

        frame.render_widget(
            Paragraph::new(bottom_line).block(Block::new().padding(Padding::new(2, 0, 1, 0))),
            bottom,
        );
    }
}
