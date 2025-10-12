/*!
*   Stores the Communioty DID Secrets on an OpenPGP compatible card (E.g. Nitrokey)
*/

use crate::{
    CLI_BLUE, CLI_ORANGE, CLI_PURPLE,
    openpgp_card::{cards, print_cards},
    setup::CommunityDIDKeys,
};
use anyhow::Result;
use console::style;
use crossterm::{
    event::{self, Event},
    terminal,
};

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

    Ok(())
}
