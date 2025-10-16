/*!
*   Stores the Communioty DID Secrets on an OpenPGP compatible card (E.g. Nitrokey)
*/

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_RED,
    openpgp_card::{
        cards, factory_reset, print_cards, set_signing_touch_policy, write::write_keys_to_card,
    },
    setup::CommunityDIDKeys,
};
use anyhow::{Result, bail};
use console::{Term, style};
use crossterm::{
    event::{self, Event},
    terminal,
};
use dialoguer::{Confirm, Password, Select, theme::ColorfulTheme};
use secrecy::SecretString;

/// Handles storing secrets on an OpenPGP compatable card
/// Returns:
/// None: No Hardware token being used
/// Some(String): The card identifier of the card used
pub fn setup_hardware_token(term: &Term, keys: &CommunityDIDKeys) -> Result<Option<String>> {
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
        return Ok(None);
    } else {
        print_cards(&mut cards)?;
    }

    let mut s_card: Vec<String> = cards
        .iter_mut()
        .map(|c| {
            c.transaction()
                .unwrap()
                .application_identifier()
                .unwrap()
                .ident()
        })
        .collect();

    s_card.push("Do not use Hardware Token".to_string());

    println!();
    let selected_option = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick which card you want to write your secrets to")
        .default(0)
        .items(&s_card)
        .interact()
        .unwrap();

    if selected_option == s_card.len() - 1 {
        println!(
            "{}",
            style("Skipping hardware token setup...").color256(CLI_ORANGE)
        );
        return Ok(None);
    }

    let Some(selected_card) = cards.get_mut(selected_option) else {
        println!(
            "{}{}{}",
            style("Couldn't find card (").color256(CLI_RED),
            style(s_card.get(selected_option).unwrap()).color256(CLI_ORANGE),
            style(")").color256(CLI_RED)
        );
        bail!("Couldn't select card for writing...");
    };

    // Ask to factory reset card?
    println!(
        "\n{}",
        style(
            "It is reccomended to factory reset your hardware token to ensure a fresh and known starting point."
        ).color256(CLI_BLUE)
    );
    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Do you want to factory reset the card before writing? {}",
            style("(This will delete all existing keys on the card)").color256(CLI_ORANGE),
        ))
        .default(false)
        .interact()?
    {
        factory_reset(term, selected_card)?;
    }

    // Open the card in admin mode
    let admin_pin: SecretString = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Admin PIN")
        .allow_empty_password(true)
        .interact()
        .unwrap()
        .into();

    // Attempt to write the keys to the card
    write_keys_to_card(term, selected_card, keys, &admin_pin)?;

    // Set Touch on for the Signing Key
    println!("{}", style("Best practice is to force an interaction with the hardware token for critical operations, such as signing data.").color256(CLI_BLUE));
    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Do you want to set the SIGNING key to require touch? {}",
            style(
                "(This will require you to touch the hardware token every time you sign something)"
            )
            .color256(CLI_GREEN),
        ))
        .default(true)
        .interact()?
    {
        set_signing_touch_policy(term, selected_card, &admin_pin)?;
    } else {
        println!(
            "{}",
            style("The SIGNING key will NOT require touch.").color256(CLI_ORANGE)
        );
    }

    // Return the card identifier
    Ok(s_card.get(selected_option).map(|s| s.to_string()))
}
