/// Holds all state related info for the setup page
#[derive(Clone, Debug, Default)]
pub struct SetupPageState {
    pub active_page: SetupPages,
}

#[derive(Clone, Debug)]
pub enum SetupPages {
    Choice(ChoiceState),
    KeyRecovery,
}

impl Default for SetupPages {
    fn default() -> Self {
        SetupPages::Choice(ChoiceState::default())
    }
}

// ****************************************************************************
// Initial setup mode selection state
// ****************************************************************************

#[derive(Clone, Debug, Default)]
pub struct ChoiceState {
    pub active_panel: ChoicePanel,
}

#[derive(Default, Debug, Clone)]
pub enum ChoicePanel {
    #[default]
    Left,
    Right,
}

impl ChoicePanel {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            ChoicePanel::Left => ChoicePanel::Right,
            ChoicePanel::Right => ChoicePanel::Left,
        }
    }
}

// ****************************************************************************
// Key Recovery
// ****************************************************************************
