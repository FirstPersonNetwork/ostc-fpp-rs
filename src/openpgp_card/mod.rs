/*!
*   Handles everything todo with openpgp-card tokens
*/

use anyhow::Result;
use card_backend_pcsc::PcscBackend;
use openpgp_card::{Card, state::Open};

/// Get a list of active cards on this system
pub fn cards() -> Result<Vec<Card<Open>>> {
    let mut cards = vec![];

    for backend in PcscBackend::cards(None)? {
        let card = Card::<Open>::new(backend?)?;
        cards.push(card);
    }

    Ok(cards)
}

/// Formats the cardholder name
/// Returns None if the name is empty
pub fn format_cardholder_name(card_holder: &str) -> Option<String> {
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
