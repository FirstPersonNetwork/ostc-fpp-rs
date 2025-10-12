/*!
*   Stores the Communioty DID Secrets on an OpenPGP compatible card (E.g. Nitrokey)
*/

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_PURPLE,
    openpgp_card::{cards, format_cardholder_name},
    setup::CommunityDIDKeys,
};
use anyhow::Result;
use console::style;
use openpgp_card::{Card, state::Open};

/// Handles storing secrets on an OpenPGP compatable card
pub fn setup_hardware_token(keys: &CommunityDIDKeys) -> Result<()> {
    println!();
    println!(
        "{}",
        style("Looking for openpgp-card compatible tokens...").color256(CLI_BLUE)
    );

    // Detect cards and show
    let mut cards = cards()?;
    print_cards(&mut cards)?;

    Ok(())
}

pub fn print_cards(cards: &mut [Card<Open>]) -> Result<()> {
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
