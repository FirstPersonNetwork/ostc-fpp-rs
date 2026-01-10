use crate::{
    state_handler::{
        actions::Action,
        setup_page::{BIP32Choice, SetupPages},
        state::{ActivePage, State},
    },
    ui::{
        component::{Component, ComponentRender},
        pages::setup::render_setup_header,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
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
use tokio::sync::mpsc::UnboundedSender;

pub struct Props {
    active_page: ActivePage,
    active_choice: BIP32Choice,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        Props {
            active_page: state.active_page,
            active_choice: if let SetupPages::KeyRecovery(choice) = &state.setup_page.active_page {
                choice.active_choice.clone()
            } else {
                BIP32Choice::default()
            },
        }
    }
}

/// SetupBIP32InitializePage handles the UI and the state for how the BIP32 phrase is created
pub struct SetupBIP32InitializePage {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,
    /// State Mapped SetupPage Props
    pub props: Props,
}

impl Component for SetupBIP32InitializePage {
    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::F(10) => {
                let _ = self.action_tx.send(Action::Exit);
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                // Switch active panel
                let _ = self.action_tx.send(Action::SetupBIP32PhraseOptionSwitch(
                    self.props.active_choice.switch(),
                ));
            }
            _ => {}
        }
    }

    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
    where
        Self: Sized,
    {
        SetupBIP32InitializePage {
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
        SetupBIP32InitializePage {
            props: Props::from(state),
            // propagate the update to the child components
            ..self
        }
    }
}

// ****************************************************************************
// Primary Render function for this page
// ****************************************************************************
impl ComponentRender<()> for SetupBIP32InitializePage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let [top, middle, bottom] =
            Layout::vertical([Length(3), Min(0), Length(3)]).areas(frame.area());

        render_setup_header(frame, top, self.props.active_page);

        let block = Block::bordered()
            .fg(COLOR_BORDER)
            .padding(Padding::proportional(1))
            .title(" Step 1/4: BIP32 Seed Phrase ");

        let mut lines = vec![
            Line::styled(
                "LKMV derives individual keys from a common BIP32 seed phrase. This allows for a secure and private deterministic generation of key material from a single seed, rather than having to back up and restore seed material for every key.",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ),
            Line::default(),
            Line::styled(
                "Choose how to setup your BIP32 recovery phrase",
                Style::new().fg(COLOR_BORDER).bold(),
            ),
            Line::default(),
        ];

        // Render the active chocie
        if let BIP32Choice::Create = self.props.active_choice {
            lines.push(Line::styled(
                "[✓] Generate a new 24-word recovery phrase (recommended)",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
            lines.push(Line::styled(
                "[ ] Import an existing recovery phrase",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
        } else {
            lines.push(Line::styled(
                "[ ] Generate a new 24-word recovery phrase (recommended)",
                Style::new().fg(COLOR_TEXT_DEFAULT),
            ));
            lines.push(Line::styled(
                "[✓] Import an existing recovery phrase",
                Style::new().fg(COLOR_SUCCESS).bold(),
            ));
        }

        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("[TAB] to select", Style::new().fg(COLOR_ORANGE)),
            Span::styled("  |  [ENTER] to confirm", Style::new().fg(COLOR_BORDER)),
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
