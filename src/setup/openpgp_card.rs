/*!
*   Stores the Communioty DID Secrets on an OpenPGP compatible card (E.g. Nitrokey)
*/

use crate::setup::CommunityDIDKeys;
use anyhow::Result;

/// Handles storing secrets on an OpenPGP compatable card
pub fn setup_hardware_token(keys: &CommunityDIDKeys) -> Result<()> {
    Ok(())
}
