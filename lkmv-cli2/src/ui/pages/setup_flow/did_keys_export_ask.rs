use crossterm::event::{KeyCode, KeyEvent};
use lkmv::colors::{COLOR_BORDER, COLOR_ORANGE, COLOR_SUCCESS, COLOR_TEXT_DEFAULT};
use ratatui::{
    Frame,
    layout::{
        Constraint::{Length, Min},
        Layout,
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
// DIDKeysExportAsk
// ****************************************************************************
#[derive(Copy, Clone, Debug, Default)]
pub enum DIDKeysExportAsk {
    #[default]
    Skip,
    Export,
}
impl DIDKeysExportAsk {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            DIDKeysExportAsk::Skip => DIDKeysExportAsk::Export,
            DIDKeysExportAsk::Export => DIDKeysExportAsk::Skip,
        }
    }
}

impl DIDKeysExportAsk {
    pub fn handle_key_event(state: &mut SetupFlow, key: KeyEvent) {
        match key.code {
            KeyCode::F(10) => {
                let _ = state.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                state.did_keys_export_ask = state.did_keys_export_ask.switch();
            }
            KeyCode::Enter => {
                state.props.state.active_page = match state.did_keys_export_ask {
                    DIDKeysExportAsk::Skip => SetupPage::ProtectCodeAsk,
                    DIDKeysExportAsk::Export => SetupPage::DidKeysExportInputs,
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
            .title(" Step 4/4: Export Private DID Keys ");

        let mut lines = vec![
            Line::styled(
                "You may want to export the secret key material that your DID keys are using so that you can use the same key values in other applications or other DID's.",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::from(vec![
                Span::styled("NOTE: ", Style::new().fg(COLOR_ORANGE).bold()),
                Span::styled(
                    "You can export your active DID keys from LKMV at any point in the future if you change your mind.",
                    Style::new().fg(COLOR_ORANGE),
                ),
            ]),
            Line::from(vec![
                Span::styled("NOTE: ", Style::new().fg(COLOR_ORANGE).bold()),
                Span::styled(
                    "Keys will be exported in an armored PGP key export format.",
                    Style::new().fg(COLOR_ORANGE),
                ),
            ]),
            Line::default(),
            Line::styled(
                "Export private keys for use in other tools?",
                Style::new().fg(COLOR_BORDER).bold(),
            ),
            Line::default(),
        ];

        // Render the active chocie
        if let DIDKeysExportAsk::Skip = self {
            lines.push(Line::styled(
                "[✓] Skip for now (recommended)",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
            lines.push(Line::styled(
                "[ ] Export private DID keys",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
        } else {
            lines.push(Line::styled(
                "[ ] Skip for now (recommended)",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
            lines.push(Line::styled(
                "[✓] Export private DID keys",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
        }

        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("[TAB]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to select  |  ", Style::new().fg(COLOR_TEXT_DEFAULT)),
            Span::styled("[ENTER]", Style::new().fg(COLOR_BORDER).bold()),
            Span::styled(" to confirm", Style::new().fg(COLOR_TEXT_DEFAULT)),
        ]));

        frame.render_widget(
            Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
            middle,
        );

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
