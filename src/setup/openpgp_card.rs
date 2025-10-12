/*!
*   Stores the Communioty DID Secrets on an OpenPGP compatible card (E.g. Nitrokey)
*/

use crate::{CLI_BLUE, CLI_GREEN, CLI_PURPLE, setup::CommunityDIDKeys};
use anyhow::Result;
use card_backend_pcsc::PcscBackend;
use console::style;
use openpgp_card::{Card, state::Open};

/// Handles storing secrets on an OpenPGP compatable card
pub fn setup_hardware_token(keys: &CommunityDIDKeys) -> Result<()> {
    println!();
    println!(
        "{}",
        style("Looking for openpgp-card compatible tokens...").color256(CLI_BLUE)
    );
    let mut cards = cards()?;

    println!(
        "{} {}",
        style("Cards found:").color256(CLI_BLUE),
        style(cards.len()).color256(CLI_GREEN)
    );
    for card in cards.iter_mut() {
        let mut open_card = card.transaction()?;
        let app_identifier = open_card.application_identifier()?;
        print!(
            "{} {} {} {}",
            style("Card Identifier:").color256(CLI_BLUE),
            style(app_identifier.ident()).color256(CLI_GREEN),
            style("Found token from manufacturer:").color256(CLI_BLUE),
            style(app_identifier.manufacturer_name()).color256(CLI_GREEN),
        );

        print!(" {}", style("Card Holder Name:").color256(CLI_BLUE));
        if let Some(cardholder) = format_cardholder_name(&open_card.cardholder_name()?) {
            println!("{}", style(cardholder).color256(CLI_GREEN));
        } else {
            println!("{}", style("<NOT SET>").color256(CLI_PURPLE).blink());
        }
    }
    Ok(())
}

/// Get a list of active cards on this system
pub(crate) fn cards() -> Result<Vec<Card<Open>>> {
    let mut cards = vec![];

    for backend in PcscBackend::cards(None)? {
        let card = Card::<Open>::new(backend?)?;
        cards.push(card);
    }

    Ok(cards)
}

/// Formats the cardholder name
/// Returns None if the name is empty
fn format_cardholder_name(card_holder: &str) -> Option<String> {
    if card_holder.is_empty() {
        None
    } else {
        // cardholder name format is LAST_NAME<<FIRST_NAME<OTHER
        // NOTE: May not always contains the << Filler
        // See  ISO/IEC 7501-1 for more info

        if card_holder.contains("<<") {
            let parts: Vec<&str> = card_holder.split("<<").collect();
            let last_name = parts
                .first()
                .unwrap_or(&"")
                .replace("<", " ")
                .trim()
                .to_string();
            let first_names = parts
                .get(1)
                .unwrap_or(&"")
                .replace("<", " ")
                .trim()
                .to_string();
            Some(format!("{} {}", first_names, last_name))
        } else {
            Some(card_holder.replace("<", " ").trim().to_string())
        }
    }
}
