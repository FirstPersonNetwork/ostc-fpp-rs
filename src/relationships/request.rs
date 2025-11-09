/*!
*    Handles relationship requests
*/

use std::time::SystemTime;

use crate::{CLI_RED, config::Config};
use affinidi_tdk::{TDK, didcomm::Message};
use anyhow::{Result, bail};
use console::style;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

// ****************************************************************************
// Message body format structs
// ****************************************************************************

#[derive(Serialize, Deserialize)]
struct RelationshipRequestBody {
    reason: Option<String>,
}

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
    // Check if the remote DID exists in contacts
    let contact = if let Some(contact) = config.contacts.find_contact(respondent) {
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

    // is a local relationship-did needed?

    // Create the Relationship Request Message
    let msg = create_message_request(&config.public.community_did, &contact.did, reason)?;

    println!("DEBUG: Relationship request\n{:#?}", msg);

    Ok(())
}

fn create_message_request(from: &str, to: &str, reason: Option<&str>) -> Result<Message> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let message = Message::build(
        Uuid::new_v4().into(),
        "https://linuxfoundation.org/lkmv/1.0/relationship-request".to_string(),
        json!(RelationshipRequestBody {
            reason: reason.map(|r| r.to_string())
        }),
    )
    .from(from.to_string())
    .to(to.to_string())
    .created_time(now)
    .expires_time(60 * 60 * 48) // 48 hours
    .finalize();

    Ok(message)
}
