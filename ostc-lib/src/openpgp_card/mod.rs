/*!
*   Handles everything todo with openpgp-card tokens
*/

use crate::errors::OSTCError;
use card_backend_pcsc::PcscBackend;
use openpgp_card::{state::Open, Card};
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod crypt;

/// Get a list of active cards on this system
/// Returns a Vector containg Arc as the PGP Card struct doesn't allow copy/clone
pub fn get_cards() -> Result<Vec<Arc<Mutex<Card<Open>>>>, OSTCError> {
    let mut cards = vec![];

    for backend in PcscBackend::cards(None)
        .map_err(|e| OSTCError::Token(format!("Couldn't get list of tokens. Reason: {e}")))?
    {
        let card = Card::<Open>::new(
            backend
                .map_err(|e| OSTCError::Token(format!("Couldn't get card backend. Reason: {e}")))?,
        )
        .map_err(|e| OSTCError::Token(format!("Couldn't open card. Reason: {e}")))?;
        cards.push(Arc::new(Mutex::new(card)));
    }

    Ok(cards)
}

/// Opens a specific openpgp-card by an identifier
pub fn open_card(token_id: &str) -> Result<Card<Open>, OSTCError> {
    let cards = PcscBackend::card_backends(None)
        .map_err(|e| OSTCError::Token(format!("Couldn't get PGP cards backend: {}", e)))?;
    let card = Card::<Open>::open_by_ident(cards, token_id)
        .map_err(|e| OSTCError::Token(format!("Couldn't open card ({token_id}): {e}")))?;

    Ok(card)
}

/// Performs a factory reset on the card, erasing all keys and data
pub fn factory_reset(card: Arc<Mutex<Card<Open>>>) -> Result<(), OSTCError> {
    let mut lock = card.try_lock().unwrap();
    let mut card = lock
        .transaction()
        .map_err(|e| OSTCError::Token(format!("Couldn't get transaction for factory reset: {e}")))?;
    card.factory_reset()
        .map_err(|e| OSTCError::Token(format!("Factory reset failed: {e}")))?;

    Ok(())
}
