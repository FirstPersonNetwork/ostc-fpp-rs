use std::{rc::Rc, sync::Mutex};

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE,
    config::Config,
    log::LogFamily,
    relationships::Relationship,
    tasks::Task,
    vrc::{VRCRequest, VRCRequestReject},
};
use affinidi_tdk::{TDK, didcomm::PackEncryptedOptions};
use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

/// Handles the menu for an interactive Inbound VRC Request
pub async fn interact_vrc_inbound_request(
    tdk: &TDK,
    config: &mut Config,
    task: &Rc<Mutex<Task>>,
    request: &VRCRequest,
    relationship: &Rc<Mutex<Relationship>>,
) -> Result<bool> {
    // Show details of the VRC Request
    println!();
    let (from, from_c_did, to) = {
        let lock = relationship.lock().unwrap();
        (
            lock.remote_did.clone(),
            lock.remote_c_did.clone(),
            lock.our_did.clone(),
        )
    };

    let task_id = { task.lock().unwrap().id.clone() };

    let alias = if let Some(contact) = config.private.contacts.find_contact(&from_c_did)
        && let Some(alias) = &contact.alias
    {
        style(alias.to_string()).color256(CLI_GREEN)
    } else {
        style("NO ALIAS".to_string()).color256(CLI_ORANGE)
    };

    println!(
        "{}{} {}{}",
        style("From: alias: ").color256(CLI_BLUE),
        alias,
        style(" C-DID: ").color256(CLI_BLUE),
        style(&from_c_did).color256(CLI_PURPLE)
    );
    println!(
        "{}{}",
        style("To: ").color256(CLI_BLUE),
        style(&to).color256(CLI_PURPLE)
    );

    request.print();
    println!();

    match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Task Action?")
        .item("Accept this VRC request")
        .item("Reject this VRC request")
        .item("Delete this VRC request (Does not notify the other party)")
        .item("Return to previous menu?")
        .interact()?
    {
        0 => {
            // Accept the VRC Request
            Ok(true)
        }
        1 => {
            // Reject the VRC Request
            let reason: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt(
                    "Would you like to provide a reason for this rejection (Leave BLANK for None)?",
                )
                .allow_empty(true)
                .interact_text()
                .unwrap();

            let reason = if reason.trim().is_empty() {
                None
            } else {
                Some(reason.trim().to_string())
            };

            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Are you sure you want to reject this VRC request?")
                .default(true)
                .interact()?
            {
                let msg = VRCRequestReject::create_message(&to, &from, &task_id, reason.clone())?;

                // Pack the message
                let (msg, _) = msg
                    .pack_encrypted(
                        &to,
                        Some(&from),
                        Some(&from),
                        tdk.did_resolver(),
                        &tdk.get_shared_state().secrets_resolver,
                        &PackEncryptedOptions {
                            forward: false,
                            ..Default::default()
                        },
                    )
                    .await?;

                let atm = tdk.atm.clone().unwrap();
                atm.forward_and_send_message(
                    &config.community_did.profile,
                    false,
                    &msg,
                    None,
                    &config.public.mediator_did,
                    to.as_str(),
                    None,
                    None,
                    false,
                )
                .await?;

                config.private.tasks.remove(&task_id);
                config.public.logs.insert(
                    LogFamily::Task,
                    format!(
                        "Rejected VRC request from remote DID({}) Task ID({}) Reason: {}",
                        from,
                        task_id,
                        reason.as_deref().unwrap_or("NO REASON PROVIDED")
                    ),
                );

                println!();
                println!(
                    "{}{}",
                    style("✅ Succesfully sent VRC Request Rejection to ").color256(CLI_GREEN),
                    style(to).color256(CLI_PURPLE)
                );

                Ok(true)
            } else {
                // Cancel rejection
                Ok(false)
            }
        }
        2 => {
            // Delete the VRC Request
            println!("{}", style("When you delete a VRC request, no response is sent back to the initiator of the request. Deleting acts as a silent ignore...").color256(CLI_BLUE));
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Are you sure you want to DELETE this VRC request?")
                .default(false)
                .interact()?
            {
                config.private.tasks.remove(&task_id);
                config.public.logs.insert(
                    LogFamily::Task,
                    format!(
                        "Deleted VRC request from remote DID({}) Task ID({})",
                        from_c_did, task_id
                    ),
                );
                Ok(true)
            } else {
                Ok(false)
            }
        }
        3 => Ok(false),

        _ => Ok(false),
    }
}
