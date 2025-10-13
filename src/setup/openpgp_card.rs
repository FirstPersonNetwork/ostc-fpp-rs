/*!
*   Stores the Communioty DID Secrets on an OpenPGP compatible card (E.g. Nitrokey)
*/

use crate::{
    CLI_BLUE, CLI_ORANGE, CLI_PURPLE, CLI_RED,
    openpgp_card::{cards, print_cards, write::write_keys_to_card},
    setup::CommunityDIDKeys,
};
use anyhow::{Result, bail};
use console::style;
use crossterm::{
    event::{self, Event},
    terminal,
};
use dialoguer::{Select, theme::ColorfulTheme};

/// Handles storing secrets on an OpenPGP compatable card
pub fn setup_hardware_token(keys: &CommunityDIDKeys) -> Result<()> {
    println!();

    println!(
        "{} {}",
        style("If you want to use hardware tokens, please ensure they are plugged in now!")
            .color256(CLI_BLUE),
        style("(press any key to continue)")
            .color256(CLI_PURPLE)
            .blink()
    );
    terminal::enable_raw_mode()?;
    loop {
        // Read the next event
        match event::read()? {
            // If it's a key event and a key press
            Event::Key(key_event) if key_event.kind == event::KeyEventKind::Press => {
                break;
            }
            _ => {} // Ignore other events (mouse, resize, etc.)
        }
    }
    // Disable raw mode when done
    terminal::disable_raw_mode()?;

    println!(
        "{}",
        style("Looking for openpgp-card compatible tokens...").color256(CLI_BLUE)
    );

    // Detect cards and show
    let mut cards = cards()?;
    if cards.is_empty() {
        println!(
            "{}",
            style("No hardware tokens were found!").color256(CLI_ORANGE)
        );
        return Ok(());
    } else {
        print_cards(&mut cards)?;
    }

    let s_card: Vec<String> = cards
        .iter_mut()
        .map(|c| {
            c.transaction()
                .unwrap()
                .application_identifier()
                .unwrap()
                .ident()
        })
        .collect();

    println!();
    let selected_option = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick which card you want to write your secrets to")
        .default(0)
        .items(&s_card)
        .interact()
        .unwrap();

    let Some(selected_card) = cards.get_mut(selected_option) else {
        println!(
            "{}{}{}",
            style("Couldn't find card (").color256(CLI_RED),
            style(s_card.get(selected_option).unwrap()).color256(CLI_ORANGE),
            style(")").color256(CLI_RED)
        );
        bail!("Couldn't select card for writing...");
    };

    // Attempt to write the keys to the card
    write_keys_to_card(selected_card, keys)?;

    Ok(())
}
