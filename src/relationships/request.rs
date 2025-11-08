/*!
*    Handles relationship requests
*/

use affinidi_tdk::TDK;
use anyhow::{Result, bail};
use console::style;

use crate::{CLI_RED, config::Config};

/// Creates a new Relationship Request and
/// tdk: Trust Development Kit instance
/// config: mutable reference to the configuration
/// respondent: the remote alias or DID to create a relationship with
/// alias: optional alias for the remote DID if it doesn't exist in contacts
/// reason: Optional reason for creating this relationship request
/// generate_did: whether to generate a new local R-DID for the relationship
pub async fn create_request(
    tdk: TDK,
    config: &mut Config,
    respondent: &str,
    alias: Option<String>,
    reason: Option<&str>,
    generate_did: bool,
) -> Result<()> {
    println!("TIMTAM: {}", respondent);
    // Check if the remote DID exists in contacts
    let contact = if let Some(contact) = config.contacts.find_contact(respondent) {
        println!("TIMTAM: FOUND",);
        contact
    } else {
        // Create a new contact
        if respondent.starts_with("did:") {
            config
                .contacts
                .add_contact(&tdk, respondent, alias, true)
                .await?
        } else {
            println!(
                "{}",
                style(format!(
                    "ERROR: No contact found for '{}'. Please provide a valid DID or add the contact first.",
                    respondent
                )).color256(CLI_RED)
            );
            bail!("Not a valid DID");
        }
    };
    println!("TIMTAM: FINAL");

    Ok(())
}
