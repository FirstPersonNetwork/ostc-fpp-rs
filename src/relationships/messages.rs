/*!
*    Handles relationship requests
*/

use crate::{
    CLI_GREEN, CLI_PURPLE, CLI_RED,
    config::Config,
    log::LogFamily,
    relationships::{
        Relationship, RelationshipRequestBody, RelationshipState, create_relationship_did,
    },
    tasks::TaskType,
};
use affinidi_tdk::{
    TDK,
    didcomm::{Message, PackEncryptedOptions},
    messaging::profiles::ATMProfile,
};
use anyhow::{Result, bail};
use chrono::Utc;
use console::style;
use serde_json::json;
use std::{rc::Rc, time::SystemTime};
use uuid::Uuid;

/// Creates a new Relationship Request and send it to the remote party
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
    let contact = if let Some(contact) = config.private.contacts.find_contact(respondent) {
        contact
    } else {
        // Create a new contact
        if respondent.starts_with("did:") {
            config
                .private
                .contacts
                .add_contact(&tdk, respondent, alias, true, &mut config.public.logs)
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

    let atm = tdk.atm.clone().unwrap();

    // is a local relationship-did needed?
    let (our_did, our_profile) = if generate_did {
        let mediator = config.public.mediator_did.clone();
        let r_did = Rc::new(create_relationship_did(&tdk, config, &mediator).await?);
        println!(
            "{}{}{}{}",
            style("Generated new Relationship DID for contact ").color256(CLI_GREEN),
            style(contact.alias.as_deref().unwrap_or(&contact.did)).color256(CLI_PURPLE),
            style(" :: ").color256(CLI_GREEN),
            style(&r_did).color256(CLI_PURPLE)
        );
        let profile = ATMProfile::new(&atm, None, r_did.to_string(), Some(mediator)).await?;
        (r_did, atm.profile_add(&profile, false).await?)
    } else {
        (
            config.public.community_did.clone(),
            config.community_did.profile.clone(),
        )
    };

    // Create the Relationship Request Message
    let msg = create_message_request(&our_did, &contact.did, reason)?;
    let msg_id = Rc::new(msg.id.clone());

    // Pack the message
    let (msg, _) = msg
        .pack_encrypted(
            &contact.did,
            Some(&our_did),
            Some(&our_did),
            tdk.did_resolver(),
            &tdk.get_shared_state().secrets_resolver,
            &PackEncryptedOptions {
                forward: false,
                ..Default::default()
            },
        )
        .await?;

    atm.forward_and_send_message(
        &our_profile,
        false,
        &msg,
        None,
        &config.public.mediator_did,
        &contact.did,
        None,
        None,
        false,
    )
    .await?;

    config.private.relationships.relationships.insert(
        contact.did.clone(),
        Rc::new(Relationship {
            task_id: msg_id.clone(),
            our_did: our_did.clone(),
            remote_c_did: contact.did.clone(),
            remote_did: contact.did.clone(),
            created: Utc::now(),
            state: RelationshipState::RequestSent,
        }),
    );

    config
        .private
        .tasks
        .new_task(&msg_id, TaskType::RelationshipRequestOutbound);

    println!();
    println!(
        "{}{}",
        style("✅ Succesfully sent Relationship Request to ").color256(CLI_GREEN),
        style(&contact.did).color256(CLI_PURPLE)
    );

    config.public.logs.insert(
        LogFamily::Relationship,
        &format!(
            "Relationship requested: remote DID({}) Task ID({})",
            &contact.did, &msg_id
        ),
    );

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

fn create_message_rejected(from: &str, to: &str, reason: Option<&str>) -> Result<Message> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let message = Message::build(
        Uuid::new_v4().into(),
        "https://linuxfoundation.org/lkmv/1.0/relationship-request-reject".to_string(),
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
