/*!
*    Verifiable Relationship Credential Entry Point
*/

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_RED, CLI_WHITE,
    config::Config,
    log::LogFamily,
    relationships::Relationship,
    tasks::{Task, TaskType},
    vrc::{
        Vrc, VrcRequest,
        remove::remove_vrc_by_id,
        visual::{show_vrc_by_id, vrcs_show_all, vrcs_show_relationship},
    },
};
use affinidi_tdk::{
    TDK,
    didcomm::{Message, PackEncryptedOptions},
};
use anyhow::{Result, bail};
use clap::ArgMatches;
use console::style;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use std::{collections::HashMap, rc::Rc, sync::Mutex};

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
        Some(("list", sub_args)) => {
            if let Some(remote) = sub_args.get_one::<String>("remote") {
                if let Some(contact) = config.private.contacts.find_contact(&Rc::new(remote)) {
                    vrcs_show_relationship(&contact.did, config);
                } else {
                    println!(
                        "{}{}",
                        style("WARN: Couldn't find any matching contact/relationship for: ")
                            .color256(CLI_ORANGE),
                        style(remote).color256(CLI_WHITE)
                    );
                }
            } else {
                vrcs_show_all(config);
            }
        }
        Some(("show", sub_args)) => {
            if let Some(id) = sub_args.get_one::<String>("id") {
                show_vrc_by_id(config, id);
            } else {
                println!(
                    "{}",
                    style("WARN: You must specify a VRC ID!").color256(CLI_ORANGE)
                );
            }
        }
        Some(("remove", sub_args)) => {
            if let Some(id) = sub_args.get_one::<String>("id") {
                remove_vrc_by_id(config, id);
                config.save(profile)?;
            } else {
                println!(
                    "{}",
                    style("WARN: You must specify a VRC ID!").color256(CLI_ORANGE)
                );
            }
        }
        _ => {
            println!(
                "{} {}",
                style("ERROR:").color256(CLI_RED),
                style("No valid vrcs subcommand was used. Use --help for more information.")
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
        style("Select a relationship to request a VRC:").color256(CLI_BLUE)
    );
    let Some(relationship) = select_relationship(config) else {
        return Ok(false);
    };

    let request_body = generate_vrc_request_body(
        &relationship,
        &config.public.community_did,
        &config.public.friendly_name,
    )?;

    request_body.print();

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Send VRC request?")
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
        let task = config
            .private
            .tasks
            .new_task(&msg_id, TaskType::VRCRequestOutbound { relationship });
        let task_id = { task.lock().unwrap().id.clone() };

        config.public.logs.insert(
            LogFamily::Relationship,
            format!("Requested a VRC from ({}) Task ID ({})", to_c_did, task_id),
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
    if relationships.is_empty() {
        println!("{}", style("No relationships found.").color256(CLI_ORANGE));
        println!();
        println!(
            "{} \n{}",
            style("To create a relationship, run:").color256(CLI_BLUE),
            style("lkmv relationships request --respondent <did> --alias <respondent-alias>")
                .color256(CLI_BLUE)
        );
        return None;
    }

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
        .with_prompt("Select from the list (press ESC or q to quit): ")
        .items(items)
        .interact_opt()
        .unwrap();

    if let Some(selected) = selected {
        Some(relationships[selected].clone())
    } else {
        println!(
            "{}",
            style("No relationship selected.").color256(CLI_ORANGE)
        );
        None
    }
}

fn generate_vrc_request_body(
    relationship: &Rc<Mutex<Relationship>>,
    our_c_did: &Rc<String>,
    friendly_name: &str,
) -> Result<VrcRequest> {
    let reason: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter a reason for the VRC request (optional, press Enter to skip)")
        .allow_empty(true)
        .interact_text()?;

    let reason = if reason.trim().is_empty() {
        None
    } else {
        Some(reason.trim().to_string())
    };

    println!();
    println!(
        "{} {}",
        style("Your current human-readable name: ").color256(CLI_BLUE),
        style(friendly_name).color256(CLI_GREEN)
    );

    let name = match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Include your human-readable name in the VRC request?")
        .items([
            "Yes, include my name",
            "Change my name",
            "Do not include a name",
        ])
        .default(0)
        .interact()?
    {
        0 => Some(friendly_name.to_string()),
        1 => Some(
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter the name to include in the VRC request")
                .interact_text()
                .unwrap(),
        ),
        2 => None,
        _ => Some(friendly_name.to_string()),
    };

    let lock = relationship.lock().unwrap();
    let include_r_did = if &lock.our_did != our_c_did {
        println!(
            "{}{}",
            style("You are using an R-DID for this relationship: ").color256(CLI_BLUE),
            style(&lock.our_did).color256(CLI_PURPLE)
        );
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Include R-DID in alsoKnownAs?")
            .default(false)
            .interact()?
    } else {
        false
    };

    let type_: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Suggest a relationship type (e.g., Coworker, Peer, or a Relationship Type URI) \n   (optional, press Enter to skip)")
        .allow_empty(true)
        .interact_text()?;

    let type_ = if type_.trim().is_empty() {
        None
    } else {
        Some(type_.trim().to_string())
    };

    let start_date = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Request to include a start date in the VRC request?")
        .default(true)
        .interact()?;

    let end_date = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Request to include an end date in the VRC request?")
        .default(false)
        .interact()?;

    Ok(VrcRequest {
        reason,
        include_r_did,
        type_,
        start_date,
        end_date,
        name,
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

/// Handles an inbound VRC Issued Message
/// If related to a task, updates the Task information
/// If not, then creates a new task for the user to accept or reject the VRC
pub async fn handle_inbound_vrc_issued(
    tdk: &TDK,
    config: &mut Config,
    message: &Message,
) -> Result<Vrc> {
    // Valid VRC structure?
    let vrc: Vrc = match serde_json::from_value(message.body.clone()) {
        Ok(vrc) => vrc,
        Err(e) => {
            println!(
                "{}{}",
                style("ERROR: VRC issued body is not a valid VRC! Reason: ").color256(CLI_RED),
                style(e).color256(CLI_ORANGE)
            );
            bail!("Invalid VRC Body");
        }
    };

    let Some(proof) = vrc.proof.clone() else {
        println!(
            "{}",
            style("ERROR: VRC issued does not contain a proof!").color256(CLI_RED)
        );
        bail!("VRC Missing Proof");
    };

    let check_vrc = Vrc {
        proof: None,
        ..vrc.clone()
    };

    // Check the proof of the VRC
    match tdk.verify_data(&check_vrc, None, &proof).await {
        Ok(r) => {
            if r.verified {
                println!(
                    "{}",
                    style("✅ VRC proof verified successfully").color256(CLI_GREEN)
                );
            } else {
                println!(
                    "{}",
                    style("VRC Proof failed integrity checks.").color256(CLI_RED)
                );
                bail!("VRC Failed Data Integrity Check");
            }
        }
        Err(e) => {
            println!(
                "{}{}",
                style("ERROR: VRC Failed Proof validation. Reason: ").color256(CLI_RED),
                style(e).color256(CLI_ORANGE)
            );
            bail!("VRC Proof Validation Error");
        }
    }

    if let Some(thid) = &message.thid {
        if let Some(task) = config.private.tasks.get_by_id(&Rc::new(thid.to_string())) {
            let mut lock = task.lock().unwrap();
            lock.type_ = TaskType::VRCIssued {
                vrc: Box::new(vrc.clone()),
            };
            config.public.logs.insert(
                LogFamily::Relationship,
                format!("Inbound VRC issued updated Task ID({})", thid),
            );
            return Ok(vrc);
        } else {
            println!(
                "{}{}{}",
                style("WARN: A VRC was issued to you with a task-id (").color256(CLI_ORANGE),
                style(thid).color256(CLI_RED),
                style(") that can't be found. Creating a new task instead").color256(CLI_ORANGE)
            );
        }
    }

    // No task, create a new one
    let task = config.private.tasks.new_task(
        &Rc::new(message.id.clone()),
        TaskType::VRCIssued {
            vrc: Box::new(vrc.clone()),
        },
    );

    let task_id = task.lock().unwrap().id.clone();
    println!(
        "{} {}",
        style("Issued VRC received. New task created to accept/reject this VRC. Task ID:")
            .color256(CLI_GREEN),
        style(task_id).color256(CLI_PURPLE)
    );

    Ok(vrc)
}

/// Handles the user interaction for an inbound VRC that has been issued to you
pub fn interact_vrc_inbound(
    config: &mut Config,
    task: &Rc<Mutex<Task>>,
    vrc: Box<Vrc>,
) -> Result<bool> {
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
    println!();
    println!(
        "{}{}",
        style("VRC Issued By: ").color256(CLI_BLUE),
        style(&vrc.issuer).color256(CLI_PURPLE)
    );
    println!(
        "{}",
        style("Issued VRC:").color256(CLI_BLUE).bold().underlined()
    );
    println!(
        "{}",
        style(serde_json::to_string_pretty(&vrc).unwrap()).color256(CLI_WHITE)
    );
    println!();

    Ok(
        match Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Task Action?")
            .item("Accept this VRC")
            .item("Delete this VRC")
            .item("Return to previous menu?")
            .interact()?
        {
            0 => {
                // Accept the VRC

                let relationship_c_did = if let Some(relationship) = config
                    .private
                    .relationships
                    .find_by_remote_did(&Rc::new(vrc.issuer.clone()))
                {
                    relationship.lock().unwrap().remote_c_did.clone()
                } else {
                    println!(
                        "{}{}",
                        style("ERROR: Couldn't find relationship for Task ID: ").color256(CLI_RED),
                        style(&task_id).color256(CLI_ORANGE)
                    );
                    bail!("Couldn't find relationship for VRC Task");
                };
                config
                    .private
                    .vrcs_received
                    .entry(relationship_c_did)
                    .and_modify(|hm| {
                        hm.insert(Rc::new(vrc.get_hash().unwrap()), Rc::new(*vrc.clone()));
                    })
                    .or_insert({
                        let mut hm = HashMap::new();
                        hm.insert(Rc::new(vrc.get_hash().unwrap()), Rc::new(*vrc));
                        hm
                    });

                config.private.tasks.remove(&task_id);

                config.public.logs.insert(
                    LogFamily::Relationship,
                    format!("User accepted inbound VRC issued Task ID({})", task_id),
                );
                config
                    .public
                    .logs
                    .insert(LogFamily::Task, format!("Removing Task ID({})", task_id));

                println!();
                println!(
                    "{}",
                    style("✅ VRC accepted and stored locally.").color256(CLI_GREEN)
                );
                true
            }
            1 => {
                // Delete the VRC
                if Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Are you sure you want to DELETE this VRC?")
                    .default(false)
                    .interact()?
                {
                    config.private.tasks.remove(&task_id);
                    config.public.logs.insert(
                        LogFamily::Task,
                        format!("User deleted inbound VRC issued Task ID({})", task_id),
                    );
                    println!(
                        "{}",
                        style("VRC deleted. No notification is sent to the issuer.")
                            .color256(CLI_ORANGE)
                    );
                    true
                } else {
                    false
                }
            }
            _ => false,
        },
    )
}
