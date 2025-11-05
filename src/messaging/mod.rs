/*!
*   Everything to do with DIDComm messaging is contained within this module.
*/

use crate::config::Config;
use affinidi_tdk::{TDK, messaging::protocols::Protocols};
use anyhow::Result;

/// Pings the mediator to check connectivity
/// uses the community-DID as the TDK/ATM Profile
pub async fn ping_mediator(tdk: &mut TDK, config: &Config) -> Result<()> {
    let atm = tdk.atm.clone().unwrap();

    let protocols = Protocols::new();

    protocols
        .trust_ping
        .send_ping(
            &atm,
            &config.community_did.profile,
            &config.public.mediator_did,
            true,
            true,
            true,
        )
        .await?;

    Ok(())
}
