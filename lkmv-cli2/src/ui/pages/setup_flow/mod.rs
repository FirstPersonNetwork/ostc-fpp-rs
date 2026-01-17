use crate::{
    state_handler::{
        actions::Action,
        setup_sequence::{SetupPage, SetupState},
        state::State,
    },
    ui::{
        component::{Component, ComponentRender},
        pages::setup_flow::{
            bip32_ask::BIP32PhraseAskChoice, bip32_import::BIP32PhraseImport,
            bip32_show::BIP32PhraseShow, config_import::ConfigImport, did_keys_ask::DIDKeysAsk,
            did_keys_export_ask::DIDKeysExportAsk, did_keys_export_inputs::DIDKeysExportInputs,
            did_keys_export_show::DIDKeysExportShow, did_keys_show::DIDKeysShow,
            mediator_ask::MediatorAsk, mediator_custom::MediatorCustom, start_ask::StartAskPanel,
            unlock_code_ask::UnlockCodeAsk, unlock_code_set::UnlockCodeSet,
            unlock_code_warn::UnlockCodeWarn, username::UserName,
        },
    },
};
use crossterm::event::{KeyEvent, KeyEventKind};
use lkmv::colors::{
    COLOR_BORDER, COLOR_DARK_GRAY, COLOR_ORANGE, COLOR_SUCCESS, COLOR_TEXT_DEFAULT,
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

pub mod bip32_ask;
pub mod bip32_import;
pub mod bip32_show;
pub mod config_import;
pub mod did_keys_ask;
pub mod did_keys_export_ask;
pub mod did_keys_export_inputs;
pub mod did_keys_export_show;
pub mod did_keys_show;
pub mod mediator_ask;
pub mod mediator_custom;
pub mod start_ask;
pub mod unlock_code_ask;
pub mod unlock_code_set;
pub mod unlock_code_warn;
pub mod username;

/// Handles the Setup Flow sequence
pub struct SetupFlow {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,

    // Local state
    pub start_ask: StartAskPanel,
    pub config_import: ConfigImport,

    pub bip32_ask: BIP32PhraseAskChoice,
    pub bip32_show: BIP32PhraseShow,
    pub bip32_import: BIP32PhraseImport,

    pub did_keys_ask: DIDKeysAsk,
    pub did_keys_show: DIDKeysShow,

    pub did_keys_export_ask: DIDKeysExportAsk,
    pub did_keys_export_inputs: DIDKeysExportInputs,
    pub did_keys_export_show: DIDKeysExportShow,

    pub unlock_code_ask: UnlockCodeAsk,
    pub unlock_code_warn: UnlockCodeWarn,
    pub unlock_code_set: UnlockCodeSet,

    pub mediator_ask: MediatorAsk,
    pub mediator_custom: MediatorCustom,

    pub username: UserName,

    /// State Mapped MainPage Props
    pub props: Props,
}

pub struct Props {
    pub state: SetupState,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        Props {
            state: state.setup.clone(),
        }
    }
}

impl Component for SetupFlow {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
    where
        Self: Sized,
    {
        SetupFlow {
            action_tx: action_tx.clone(),

            start_ask: StartAskPanel::default(),
            config_import: ConfigImport::default(),
            bip32_ask: BIP32PhraseAskChoice::default(),
            bip32_show: BIP32PhraseShow::default(),
            bip32_import: BIP32PhraseImport::default(),
            did_keys_ask: DIDKeysAsk::default(),
            did_keys_show: DIDKeysShow::default(),
            did_keys_export_ask: DIDKeysExportAsk::default(),
            did_keys_export_inputs: DIDKeysExportInputs::default(),
            did_keys_export_show: DIDKeysExportShow::default(),
            unlock_code_ask: UnlockCodeAsk::default(),
            unlock_code_warn: UnlockCodeWarn::default(),
            unlock_code_set: UnlockCodeSet::default(),
            mediator_ask: MediatorAsk::default(),
            mediator_custom: MediatorCustom::default(),
            username: UserName::default(),

            // set the props
            props: Props::from(state),
        }
        .move_with_state(state)
    }

    fn move_with_state(self, state: &State) -> Self
    where
        Self: Sized,
    {
        SetupFlow {
            props: Props::from(state),
            // propagate the update to the child components
            ..self
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match self.props.state.active_page {
            SetupPage::StartAsk => StartAskPanel::handle_key_event(self, key),
            SetupPage::ConfigImport => ConfigImport::handle_key_event(self, key),
            SetupPage::BIP32PhraseAsk => BIP32PhraseAskChoice::handle_key_event(self, key),
            SetupPage::BIP32PhraseShow => BIP32PhraseShow::handle_key_event(self, key),
            SetupPage::BIP32PhraseImport => BIP32PhraseImport::handle_key_event(self, key),
            SetupPage::DIDKeysAsk => DIDKeysAsk::handle_key_event(self, key),
            SetupPage::DIDKeysShow => DIDKeysShow::handle_key_event(self, key),
            SetupPage::DidKeysExportAsk => DIDKeysExportAsk::handle_key_event(self, key),
            SetupPage::DidKeysExportInputs => DIDKeysExportInputs::handle_key_event(self, key),
            SetupPage::DidKeysExportShow => DIDKeysExportShow::handle_key_event(self, key),
            SetupPage::UnlockCodeAsk => UnlockCodeAsk::handle_key_event(self, key),
            SetupPage::UnlockCodeWarn => UnlockCodeWarn::handle_key_event(self, key),
            SetupPage::UnlockCodeSet => UnlockCodeSet::handle_key_event(self, key),
            SetupPage::MediatorAsk => MediatorAsk::handle_key_event(self, key),
            SetupPage::MediatorCustom => MediatorCustom::handle_key_event(self, key),
            SetupPage::UserName => UserName::handle_key_event(self, key),
            _ => {}
        }
    }
}

// ****************************************************************************
// Render the page
// ****************************************************************************
impl ComponentRender<()> for SetupFlow {
    fn render(&self, frame: &mut Frame, _props: ()) {
        match self.props.state.active_page {
            SetupPage::StartAsk => self.start_ask.render(&self.props.state, frame),
            SetupPage::ConfigImport => self.config_import.render(&self.props.state, frame),
            SetupPage::BIP32PhraseAsk => self.bip32_ask.render(&self.props.state, frame),
            SetupPage::BIP32PhraseShow => self.bip32_show.render(&self.props.state, frame),
            SetupPage::BIP32PhraseImport => self.bip32_import.render(&self.props.state, frame),
            SetupPage::DIDKeysAsk => self.did_keys_ask.render(&self.props.state, frame),
            SetupPage::DIDKeysShow => self.did_keys_show.render(&self.props.state, frame),
            SetupPage::DidKeysExportAsk => {
                self.did_keys_export_ask.render(&self.props.state, frame)
            }
            SetupPage::DidKeysExportInputs => {
                self.did_keys_export_inputs.render(&self.props.state, frame)
            }
            SetupPage::DidKeysExportShow => {
                self.did_keys_export_show.render(&self.props.state, frame)
            }
            SetupPage::UnlockCodeAsk => self.unlock_code_ask.render(&self.props.state, frame),
            SetupPage::UnlockCodeWarn => self.unlock_code_warn.render(&self.props.state, frame),
            SetupPage::UnlockCodeSet => self.unlock_code_set.render(&self.props.state, frame),
            SetupPage::MediatorAsk => self.mediator_ask.render(&self.props.state, frame),
            SetupPage::MediatorCustom => self.mediator_custom.render(&self.props.state, frame),
            SetupPage::UserName => self.username.render(&self.props.state, frame),
            _ => {}
        }
    }
}

/// Renders the top headline for the setup pages
pub fn render_setup_header(frame: &mut Frame, rect: Rect, state: &SetupState) {
    let mut line1 = Line::default();
    let mut step = 0;

    if let SetupPage::StartAsk = state.active_page {
        step = 1;
        line1.push_span(Span::styled(
            "● Choice",
            Style::new().fg(COLOR_ORANGE).bold(),
        ));
    } else {
        line1.push_span(Span::styled("✓ Choice", Style::new().fg(COLOR_SUCCESS)));
    }

    if let SetupPage::BIP32PhraseAsk
    | SetupPage::BIP32PhraseShow
    | SetupPage::BIP32PhraseImport
    | SetupPage::DIDKeysAsk
    | SetupPage::DIDKeysShow
    | SetupPage::DidKeysExportAsk
    | SetupPage::DidKeysExportInputs
    | SetupPage::DidKeysExportShow = state.active_page
    {
        step = 2;
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled(
            "● Key Management",
            Style::new().fg(COLOR_ORANGE).bold(),
        ));
        line1.push_span(Span::styled(
            " → ○ Security",
            Style::new().fg(COLOR_DARK_GRAY),
        ));
    } else if let SetupPage::ConfigImport = state.active_page {
        step = 2;
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled(
            "● Locate Backup",
            Style::new().fg(COLOR_ORANGE).bold(),
        ));
    } else if let SetupPage::UnlockCodeAsk | SetupPage::UnlockCodeSet | SetupPage::UnlockCodeWarn =
        state.active_page
    {
        step = 3;
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled(
            "✓ Key Management",
            Style::new().fg(COLOR_SUCCESS),
        ));
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled(
            "● Security",
            Style::new().fg(COLOR_ORANGE).bold(),
        ));
    } else if let SetupPage::MediatorAsk | SetupPage::MediatorCustom = state.active_page {
        step = 4;
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled(
            "✓ Key Management",
            Style::new().fg(COLOR_SUCCESS),
        ));
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled("✓ Security", Style::new().fg(COLOR_SUCCESS)));
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled(
            "● Messaging",
            Style::new().fg(COLOR_ORANGE).bold(),
        ));
    } else if let SetupPage::UserName = state.active_page {
        step = 5;
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled(
            "✓ Key Management",
            Style::new().fg(COLOR_SUCCESS),
        ));
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled("✓ Security", Style::new().fg(COLOR_SUCCESS)));
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled("✓ Messaging", Style::new().fg(COLOR_SUCCESS)));
        line1.push_span(Span::styled(" → ", Style::new().fg(COLOR_TEXT_DEFAULT)));
        line1.push_span(Span::styled(
            "● Identity",
            Style::new().fg(COLOR_ORANGE).bold(),
        ));
    } else {
        line1.push_span(Span::styled(
            " → ○ Key Management → ○ Security → ○ Messaging",
            Style::new().fg(COLOR_DARK_GRAY),
        ));
    }

    line1.push_span(Span::styled(
        " → ○ Identity → ○ Verify ",
        Style::new().fg(COLOR_DARK_GRAY),
    ));

    let line2 = Line::from(Span::styled(
        format!("Section {}/6", step),
        Style::new().fg(COLOR_BORDER),
    ));

    frame.render_widget(
        Paragraph::new(vec![line2, line1])
            .alignment(Alignment::Left)
            .block(Block::new().padding(Padding::new(2, 0, 0, 0))),
        rect,
    );
}
