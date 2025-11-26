/*!
*    Verifiable Relationship Credential Entry Point
*/

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_RED,
    config::Config,
    log::LogFamily,
    relationships::Relationship,
    tasks::{Task, TaskType},
    vrc::VRCRequest,
};
use affinidi_tdk::{TDK, didcomm::PackEncryptedOptions};
use anyhow::{Result, bail};
use clap::ArgMatches;
use console::style;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use std::{rc::Rc, sync::Mutex};

/// Primary entry point for VRCs interactions
pub async fn vrcs_entry(
    tdk: TDK,
    config: &mut Config,
    profile: &str,
    args: &ArgMatches,
) -> Result<()> {
    match args.subcommand() {
        Some(("request", _)) => {
            if vrcs_interactive_request(&tdk, config).await? {
                config.save(profile)?;
            }
        }
        _ => {
            println!(
                "{} {}",
                style("ERROR:").color256(CLI_RED),
                style("No vrcs subcommand was used. Use --help for more information.")
                    .color256(CLI_ORANGE)
            );
            bail!("Invalid CLI Options");
        }
    }

    Ok(())
}

/// Interactive VRC Rquest Flow
async fn vrcs_interactive_request(tdk: &TDK, config: &mut Config) -> Result<bool> {
    println!(
        "{}",
        style("Requesting a VRC, please select the relationship you would like a VRC for:")
            .color256(CLI_BLUE)
    );
    let Some(relationship) = select_relationship(config) else {
        return Ok(false);
    };

    let request_body = generate_vrc_request_body(&relationship, &config.public.community_did)?;

    request_body.print();

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Submit VRC request?")
        .default(true)
        .interact()?
    {
        let (from, to, to_c_did) = {
            let lock = relationship.lock().unwrap();
            (
                lock.our_did.clone(),
                lock.remote_did.clone(),
                lock.remote_c_did.clone(),
            )
        };

        let profile = if from == config.public.community_did {
            &config.community_did.profile
        } else if let Some(profile) = config.atm_profiles.get(&from) {
            profile
        } else {
            println!(
                "{}{}",
                style("ERROR: Couldn't find messaging profile for local relationship DID: ")
                    .color256(CLI_RED),
                style(from).color256(CLI_ORANGE)
            );
            bail!("Couldn't find ATM Profile for R-DID");
        };

        let message = request_body.create_message(&to, &from)?;
        let msg_id = Rc::new(message.id.clone());

        // Pack the message
        let (message, _) = message
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
            profile,
            false,
            &message,
            None,
            &config.public.mediator_did,
            to.as_str(),
            None,
            None,
            false,
        )
        .await?;

        // Create Task to track response
        config
            .private
            .tasks
            .new_task(&msg_id, TaskType::VRCRequestOutbound { relationship });

        config.public.logs.insert(
            LogFamily::Relationship,
            format!("Requested a VRC from {}", to_c_did),
        );

        println!(
            "{}{}",
            style("✅ Successfully sent VRC Request. Remote DID: ").color256(CLI_GREEN),
            style(&to).color256(CLI_PURPLE)
        );

        Ok(true)
    } else {
        println!(
            "{}",
            style("VRC Request cancelled. No changes made.").color256(CLI_ORANGE)
        );
        Ok(false)
    }
}

fn select_relationship(config: &Config) -> Option<Rc<Mutex<Relationship>>> {
    let mut items: Vec<String> = Vec::new();
    let relationships = config.private.relationships.get_established_relationships();
    for r in &relationships {
        let lock = r.lock().unwrap();
        let alias = if let Some(contact) = config.private.contacts.contacts.get(&lock.remote_c_did)
            && let Some(alias) = &contact.alias
        {
            alias.to_string()
        } else {
            "N/A".to_string()
        };

        items.push(format!("{} :: {}", alias, lock.remote_c_did));
    }

    let selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select relationship (ESC or q to quit)")
        .items(items)
        .interact_opt()
        .unwrap();

    if let Some(selected) = selected {
        Some(relationships[selected].clone())
    } else {
        println!(
            "{}",
            style("No relationship requested.").color256(CLI_ORANGE)
        );
        None
    }
}

fn generate_vrc_request_body(
    relationship: &Rc<Mutex<Relationship>>,
    our_c_did: &Rc<String>,
) -> Result<VRCRequest> {
    let reason: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Optional: Enter a reason for the VRC request (or leave blank to skip)")
        .allow_empty(true)
        .interact_text()?;

    let reason = if reason.trim().is_empty() {
        None
    } else {
        Some(reason.trim().to_string())
    };

    let lock = relationship.lock().unwrap();
    let include_r_did = if &lock.our_did != our_c_did {
        println!(
            "{}{}",
            style("You are using an r-did for this relationship: ").color256(CLI_BLUE),
            style(&lock.our_did).color256(CLI_PURPLE)
        );
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Include r_did in alsoKnownAs?")
            .default(false)
            .interact()?
    } else {
        false
    };

    let type_: Option<String> = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to suggest a Relationship Type URI?")
        .default(false)
        .interact()?
    {
        let r_type: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter the Relationship Type URI")
            .interact_text()?;
        Some(r_type)
    } else {
        None
    };

    let start_date = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to request including a start date?")
        .default(true)
        .interact()?;

    let end_date = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to request including an end date?")
        .default(false)
        .interact()?;

    Ok(VRCRequest {
        reason,
        include_r_did,
        type_,
        start_date,
        end_date,
    })
}

/// Interactive menu to manage an outbound VRC request
pub fn interact_vrc_outbound_request(
    config: &mut Config,
    task: &Rc<Mutex<Task>>,
    relationship: &Rc<Mutex<Relationship>>,
) -> Result<bool> {
    let to_c_did = { relationship.lock().unwrap().remote_c_did.clone() };
    let (task_id, task_created) = {
        let lock = task.lock().unwrap();
        (lock.id.clone(), lock.created)
    };

    println!(
        "{}{} {}{}",
        style("Task ID: ").color256(CLI_BLUE),
        style(&task_id).color256(CLI_GREEN),
        style("Created: ").color256(CLI_BLUE),
        style(task_created).color256(CLI_GREEN)
    );
    println!(
        "{}{}",
        style("VRC Request Sent To: ").color256(CLI_BLUE),
        style(&to_c_did).color256(CLI_PURPLE)
    );
    println!();

    match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Task Action?")
        .item("Delete this VRC request")
        .item("Return to previous menu?")
        .interact()?
    {
        0 => {
            // Delete this task
            println!("{}", style("When you delete a VRC request, no notification is sent to the remote DID. This means you may still receive a VRC in the future, it is safe to delete the VRC if one arrives.").color256(CLI_BLUE));
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Are you sure you want to DELETE this VRC request?")
                .default(false)
                .interact()?
            {
                config.private.tasks.remove(&task_id);
                config.public.logs.insert(
                    LogFamily::Task,
                    format!(
                        "Deleted VRC request to remote DID({}) Task ID({})",
                        to_c_did, task_id
                    ),
                );
                Ok(true)
            } else {
                Ok(false)
            }
        }
        1 => Ok(false),
        _ => Ok(false),
    }
}
