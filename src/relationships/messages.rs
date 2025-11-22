/*!
*    Handles relationship requests
*/

use crate::{
    CLI_GREEN, CLI_PURPLE, CLI_RED,
    config::Config,
    log::LogFamily,
    relationships::{
        Relationship, RelationshipRejectBody, RelationshipRequestBody, RelationshipState,
        create_relationship_did,
    },
    tasks::TaskType,
};
use affinidi_tdk::{
    TDK,
    didcomm::{Message, PackEncryptedOptions},
};
use anyhow::{Result, bail};
use chrono::Utc;
use console::style;
use serde_json::json;
use std::{rc::Rc, sync::Mutex, time::SystemTime};
use uuid::Uuid;

/// Creates a new Relationship Request and send it to the remote party
/// tdk: Trust Development Kit instance
/// config: mutable reference to the configuration
/// respondent: the remote alias or DID to create a relationship with
/// alias: optional alias for the remote DID if it doesn't exist in contacts
/// reason: Optional reason for creating this relationship request
/// generate_did: whether to generate a new local R-DID for the relationship
pub async fn create_send_request(
    tdk: &TDK,
    config: &mut Config,
    respondent: &str,
    alias: String,
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
                .add_contact(tdk, respondent, Some(alias), true, &mut config.public.logs)
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
    let r_did = if generate_did {
        let mediator = config.public.mediator_did.clone(); // Clone so we can borrow config
        // as mutable below
        let r_did = Rc::new(create_relationship_did(tdk, config, &mediator).await?);
        println!(
            "{}{}{}{}",
            style("Generated new Relationship DID for contact ").color256(CLI_GREEN),
            style(contact.alias.as_deref().unwrap_or(&contact.did)).color256(CLI_PURPLE),
            style(" :: ").color256(CLI_GREEN),
            style(&r_did).color256(CLI_PURPLE)
        );
        r_did
    } else {
        config.public.community_did.clone()
    };

    // Create the Relationship Request Message
    let msg = create_message_request(&config.public.community_did, &contact.did, reason, &r_did)?;
    let msg_id = Rc::new(msg.id.clone());

    // Pack the message
    let (msg, _) = msg
        .pack_encrypted(
            &contact.did,
            Some(&config.public.community_did),
            Some(&config.public.community_did),
            tdk.did_resolver(),
            &tdk.get_shared_state().secrets_resolver,
            &PackEncryptedOptions {
                forward: false,
                ..Default::default()
            },
        )
        .await?;

    atm.forward_and_send_message(
        &config.community_did.profile,
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
        Rc::new(Mutex::new(Relationship {
            task_id: msg_id.clone(),
            our_did: r_did.clone(),
            remote_c_did: contact.did.clone(),
            remote_did: contact.did.clone(),
            created: Utc::now(),
            state: RelationshipState::RequestSent,
        })),
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
        format!(
            "Relationship requested: remote DID({}) Task ID({})",
            &contact.did, &msg_id
        ),
    );

    Ok(())
}

/// Creates the initial relationship request message
/// from: initiator C-DID
/// to: Respondent C-DID
/// reason: Optional reason for the relationship request
/// our_did: What DID to use for this relationship after creation (C-DID or R-DID
fn create_message_request(
    from: &str,
    to: &str,
    reason: Option<&str>,
    our_did: &Rc<String>,
) -> Result<Message> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let message = Message::build(
        Uuid::new_v4().into(),
        "https://linuxfoundation.org/lkmv/1.0/relationship-request".to_string(),
        json!(RelationshipRequestBody {
            reason: reason.map(|r| r.to_string()),
            did: our_did.to_string(),
        }),
    )
    .from(from.to_string())
    .to(to.to_string())
    .created_time(now)
    .expires_time(60 * 60 * 48) // 48 hours
    .finalize();

    Ok(message)
}

/// Sends a Relationship rejection message to the remote party
pub async fn send_rejection(
    tdk: &TDK,
    config: &mut Config,
    respondent: &str,
    reason: Option<&str>,
    task_id: &Rc<String>,
) -> Result<()> {
    let atm = tdk.atm.clone().unwrap();

    // Create the Relationship Request rejection Message
    let msg = create_message_rejected(&config.public.community_did, respondent, reason, task_id)?;

    // Pack the message
    let (msg, _) = msg
        .pack_encrypted(
            respondent,
            Some(&config.public.community_did),
            Some(&config.public.community_did),
            tdk.did_resolver(),
            &tdk.get_shared_state().secrets_resolver,
            &PackEncryptedOptions {
                forward: false,
                ..Default::default()
            },
        )
        .await?;

    atm.forward_and_send_message(
        &config.community_did.profile,
        false,
        &msg,
        None,
        &config.public.mediator_did,
        respondent,
        None,
        None,
        false,
    )
    .await?;

    println!();
    println!(
        "{}{}",
        style("✅ Succesfully sent Relationship Request Rejection to ").color256(CLI_GREEN),
        style(respondent).color256(CLI_PURPLE)
    );

    config.public.logs.insert(
        LogFamily::Relationship,
        format!(
            "Relationship request rejected: remote DID({}) Task ID({})",
            respondent, task_id
        ),
    );

    Ok(())
}

fn create_message_rejected(
    from: &str,
    to: &str,
    reason: Option<&str>,
    task_id: &Rc<String>,
) -> Result<Message> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let message = Message::build(
        Uuid::new_v4().into(),
        "https://linuxfoundation.org/lkmv/1.0/relationship-request-reject".to_string(),
        json!(RelationshipRejectBody {
            reason: reason.map(|r| r.to_string())
        }),
    )
    .from(from.to_string())
    .to(to.to_string())
    .thid(task_id.to_string())
    .created_time(now)
    .expires_time(60 * 60 * 48) // 48 hours
    .finalize();

    Ok(message)
}
