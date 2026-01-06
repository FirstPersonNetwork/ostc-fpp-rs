/*!
*   Handles everything todo with openpgp-card tokens
*/

use crate::errors::LKMVError;
use card_backend_pcsc::PcscBackend;
use openpgp_card::{Card, state::Open};

pub mod crypt;

/// Opens a specific openpgp-card by an identifier
pub fn open_card(token_id: &str) -> Result<Card<Open>, LKMVError> {
    let cards = PcscBackend::card_backends(None)
        .map_err(|e| LKMVError::Token(format!("Couldn't get PGP cards backend: {}", e)))?;
    let card = Card::<Open>::open_by_ident(cards, token_id)
        .map_err(|e| LKMVError::Token(format!("Couldn't open card ({token_id}): {e}")))?;

    Ok(card)
}
