/// Holds all state related info for the setup page
#[derive(Clone, Debug, Default)]
pub struct SetupPageState {
    pub active_page: SetupPages,
}

#[derive(Clone, Debug)]
pub enum SetupPages {
    Choice(ChoiceState),
    KeyRecovery(KeyRecoveryState),
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

#[derive(Clone, Debug, Default)]
pub struct KeyRecoveryState {
    pub active_choice: BIP32Choice,
}

#[derive(Default, Debug, Clone)]
pub enum BIP32Choice {
    #[default]
    Create,
    Import,
}

impl BIP32Choice {
    /// Switches to the next panel when pressing <TAB>
    pub fn switch(&self) -> Self {
        match self {
            BIP32Choice::Create => BIP32Choice::Import,
            BIP32Choice::Import => BIP32Choice::Create,
        }
    }
}
