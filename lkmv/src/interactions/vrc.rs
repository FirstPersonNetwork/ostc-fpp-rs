use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_RED, CLI_WHITE,
    config::Config,
    log::LogFamily,
    relationships::Relationship,
    tasks::{Task, TaskType},
};
use affinidi_data_integrity::DataIntegrityProof;
use affinidi_tdk::{
    TDK,
    didcomm::{Message, PackEncryptedOptions},
};
use anyhow::{Result, bail};
use chrono::{DateTime, Local, SecondsFormat, Utc};
use clap::ArgMatches;
use console::style;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use lkmv::vrc::{CredentialSubject, FromSubject, ToSubject, VRCRequestReject, Vrc, VrcRequest};
use std::{collections::HashSet, rc::Rc, sync::Mutex};

pub trait Print {
    fn print(&self);
}

impl Print for VrcRequest {
    fn print(&self) {
        println!();
        println!("{}", style("VRC request details: ").color256(CLI_BLUE));

        println!();
        print!("{}", style("Request reason: ").color256(CLI_BLUE));
        if let Some(reason) = &self.reason {
            println!("{}", style(reason).color256(CLI_PURPLE));
        } else {
            println!("{}", style("NO REASON PROVIDED").color256(CLI_ORANGE));
        }

        print!(
            "{}",
            style("Suggested relationship type: ").color256(CLI_BLUE)
        );
        if let Some(type_) = &self.type_ {
            println!("{}", style(type_).color256(CLI_PURPLE));
        } else {
            println!("{}", style("NO TYPE REQUESTED").color256(CLI_ORANGE));
        }

        print!(
            "{} {} ",
            style("Human-readable name: ").color256(CLI_BLUE),
            self.name
                .as_deref()
                .map(|m| style(m).color256(CLI_GREEN))
                .unwrap_or(style("N/A").color256(CLI_ORANGE))
        );

        println!();
        print!(
            "{}",
            style("Include R-DID in alsoKnownAs: ").color256(CLI_BLUE)
        );
        if self.include_r_did {
            print!("{}", style("YES").color256(CLI_GREEN));
        } else {
            print!("{}", style("NO").color256(CLI_ORANGE));
        }

        println!();
        print!("{}", style("Start date requested: ").color256(CLI_BLUE));
        if self.start_date {
            print!("{}", style("YES").color256(CLI_GREEN));
        } else {
            print!("{}", style("NO").color256(CLI_ORANGE));
        }

        println!();
        print!("{}", style("End date requested: ").color256(CLI_BLUE));
        if self.end_date {
            println!("{}", style("YES").color256(CLI_GREEN));
        } else {
            println!("{}", style("NO").color256(CLI_ORANGE));
        }
        println!();
    }
}

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
                remove_vrc_by_id(config, &Rc::new(id.to_string()));
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
        &config.public.persona_did,
        &config.public.friendly_name,
    )?;

    request_body.print();

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Send VRC request?")
        .default(true)
        .interact()?
    {
        let (from, to, to_p_did) = {
            let lock = relationship.lock().unwrap();
            (
                lock.our_did.clone(),
                lock.remote_did.clone(),
                lock.remote_p_did.clone(),
            )
        };

        let profile = if from == config.public.persona_did {
            &config.persona_did.profile
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
            format!("Requested a VRC from ({}) Task ID ({})", to_p_did, task_id),
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
        let alias = if let Some(contact) = config.private.contacts.contacts.get(&lock.remote_p_did)
            && let Some(alias) = &contact.alias
        {
            alias.to_string()
        } else {
            "N/A".to_string()
        };

        items.push(format!("{} :: {}", alias, lock.remote_p_did));
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
    our_p_did: &Rc<String>,
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
    let include_r_did = if &lock.our_did != our_p_did {
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
    let to_p_did = { relationship.lock().unwrap().remote_p_did.clone() };
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
        style(&to_p_did).color256(CLI_PURPLE)
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
                        to_p_did, task_id
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

                let relationship_p_did = if let Some(relationship) = config
                    .private
                    .relationships
                    .find_by_remote_did(&Rc::new(vrc.issuer.clone()))
                {
                    relationship.lock().unwrap().remote_p_did.clone()
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
                    .insert(&relationship_p_did, Rc::new(*vrc));

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

/// Remove a VRC by it's ID
pub fn remove_vrc_by_id(config: &mut Config, id: &Rc<String>) -> bool {
    if let Some(vrc) = config.vrcs.get(id) {
        vrc_show(id, vrc);

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Are you sure you want to delete VRC?")
            .interact()
            .unwrap()
        {
            config.private.vrcs_received.remove_vrc(id);
            config.private.vrcs_issued.remove_vrc(id);

            config.public.logs.insert(
                LogFamily::Relationship,
                format!("User removed VRC ID: {id}"),
            );
            true
        } else {
            println!("{}", style("Aborting VRC Removal").color256(CLI_ORANGE));
            false
        }
    } else {
        println!(
            "{}{}",
            style("ERROR: No VRC found for ID: ").color256(CLI_RED),
            style(id).color256(CLI_ORANGE)
        );
        false
    }
}

/// Handles the menu for an interactive Inbound VRC Request
pub async fn interact_vrc_inbound_request(
    tdk: &TDK,
    config: &mut Config,
    task: &Rc<Mutex<Task>>,
    request: &VrcRequest,
    relationship: &Rc<Mutex<Relationship>>,
) -> Result<bool> {
    // Show details of the VRC Request
    println!();
    let (from, from_p_did, to) = {
        let lock = relationship.lock().unwrap();
        (
            lock.remote_did.clone(),
            lock.remote_p_did.clone(),
            lock.our_did.clone(),
        )
    };

    let task_id = { task.lock().unwrap().id.clone() };

    let alias = if let Some(contact) = config.private.contacts.find_contact(&from_p_did)
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
        style(" P-DID: ").color256(CLI_BLUE),
        style(&from_p_did).color256(CLI_PURPLE)
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
            Ok(handle_accept_vrcs_request(tdk, config, task, request, relationship).await?)
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
                let msg = VRCRequestReject::create_message(&from, &to, &task_id, reason.clone())?;

                let profile = if to == config.public.persona_did {
                    &config.persona_did.profile
                } else if let Some(profile) = config.atm_profiles.get(&to) {
                    profile
                } else {
                    println!(
                        "{}{}",
                        style("ERROR: Couldn't find Messaging profile for DID: ").color256(CLI_RED),
                        style(to).color256(CLI_ORANGE)
                    );
                    bail!("Couldn't find messaging profile for DID");
                };

                // Pack the message
                let (msg, _) = msg
                    .pack_encrypted(
                        &from,
                        Some(&to),
                        Some(&to),
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
                    &msg,
                    None,
                    &config.public.mediator_did,
                    from.as_str(),
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
                        from_p_did, task_id
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

/// Interactive menu for generating a VRC Response
pub async fn handle_accept_vrcs_request(
    tdk: &TDK,
    config: &mut Config,
    task: &Rc<Mutex<Task>>,
    request: &VrcRequest,
    relationship: &Rc<Mutex<Relationship>>,
) -> Result<bool> {
    // Start collecting data for VRC Response
    let (our_r_did, their_p_did, their_r_did, r_created) = {
        let lock = relationship.lock().unwrap();
        (
            lock.our_did.clone(),
            lock.remote_p_did.clone(),
            lock.remote_did.clone(),
            lock.created,
        )
    };
    let task_id = { task.lock().unwrap().id.clone() };

    println!();
    println!("{}", style("Your Information").color256(CLI_BLUE).bold());
    println!("{}", style("=================").bold().color256(CLI_BLUE));
    println!();

    println!(
        "{} {}",
        style("Your current human-readable name:").color256(CLI_BLUE),
        style(&config.public.friendly_name).color256(CLI_GREEN)
    );

    let from_name = match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Change your human-readable name in this VRC?")
        .items([
            "No, keep my name",
            "Change my name",
            "Do not include a name",
        ])
        .default(0)
        .interact()?
    {
        0 => Some(config.public.friendly_name.to_string()),
        1 => Some(
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt(
                    "Enter the name to include in this VRC (leave blank for no issuer name): ",
                )
                .allow_empty(true)
                .interact_text()
                .unwrap(),
        ),
        2 => None,
        _ => Some(config.public.friendly_name.to_string()),
    };

    if from_name.clone().unwrap().trim().is_empty() {
        println!("{}", style("No issuer name included.").color256(CLI_ORANGE));
    }

    let our_also_known_as = if our_r_did != config.public.persona_did {
        println!(
            "{}{}",
            style("This relationship is using private Relationship DIDs (R-DID): ")
                .color256(CLI_BLUE),
            style(&our_r_did).color256(CLI_PURPLE)
        );
        println!();
        println!(
            "{}{}{}",
            style("It is ")
                .color256(CLI_BLUE),
            style("NOT RECOMMENDED")
                .color256(CLI_ORANGE).bold(),
            style(" to include the R-DID in alsoKnownAs, as this is your private communication channel.")
                .color256(CLI_BLUE),
        );
        println!();
        let ask_default = if request.include_r_did {
            println!(
                "{} {}",
                style("Did the requestor request to include their R-DID?").color256(CLI_BLUE),
                style("YES").color256(CLI_GREEN)
            );
            true
        } else {
            println!(
                "{} {}",
                style("Did the requestor request to include their R-DID?").color256(CLI_BLUE),
                style("NO").color256(CLI_ORANGE)
            );
            false
        };

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Include your R-DID in your alsoKnownAs?")
            .default(ask_default)
            .interact()?
        {
            println!(
                "{}{}",
                style("You are including your R-DID in alsoKnownAs: ").color256(CLI_BLUE),
                style(&our_r_did).color256(CLI_PURPLE)
            );
            vec![our_r_did.to_string()]
        } else {
            println!(
                "{}",
                style("You are NOT including your R-DID in alsoKnownAs.").color256(CLI_BLUE)
            );
            vec![]
        }
    } else {
        vec![]
    };

    println!();
    println!(
        "{}",
        style("Requestor Information").color256(CLI_BLUE).bold()
    );
    println!(
        "{}",
        style("======================").bold().color256(CLI_BLUE)
    );
    println!();

    let their_name = if let Some(name) = &request.name {
        println!(
            "{}{}",
            style("The requestor suggested a name for themselves: ").color256(CLI_BLUE),
            style(name).color256(CLI_ORANGE)
        );
        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Use the requestor's suggested name?")
            .default(true)
            .interact()?
        {
            println!(
                "{}{}",
                style("Using the requestor's suggested name: ").color256(CLI_BLUE),
                style(name).color256(CLI_ORANGE)
            );
            Some(name.to_string())
        } else {
            let name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt(
                    "Enter a human-readable name for the requestor (leave blank for no name)",
                )
                .allow_empty(true)
                .interact_text()
                .unwrap();
            if name.trim().is_empty() {
                println!(
                    "{}",
                    style("No name will be included for the requestor.").color256(CLI_BLUE)
                );
                None
            } else {
                Some(name.trim().to_string())
            }
        }
    } else {
        let name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter a human-readable name for the requestor (leave blank for no name)")
            .allow_empty(true)
            .interact_text()
            .unwrap();
        if name.trim().is_empty() {
            println!(
                "{}",
                style("No name will be included for the requestor.").color256(CLI_BLUE)
            );
            None
        } else {
            Some(name.trim().to_string())
        }
    };

    let to_also_known_as = if their_r_did != their_p_did {
        if request.include_r_did {
            println!(
                "{}{}",
                style("The requestor has requested to include their R-DID in alsoKnownAs: ")
                    .color256(CLI_BLUE),
                style(&their_r_did).color256(CLI_PURPLE)
            );
        } else {
            println!(
                "{}",
                style("The requestor did not request to include their R-DID in alsoKnownAs.")
                    .color256(CLI_BLUE)
            );
        }
        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Include the requestor's R-DID in their alsoKnownAs?")
            .default(request.include_r_did)
            .interact()?
        {
            println!(
                "{}{}",
                style("Including the requestor's R-DID in alsoKnownAs: ").color256(CLI_BLUE),
                style(&their_r_did).color256(CLI_PURPLE)
            );
            Some(vec![their_r_did.to_string()])
        } else {
            println!(
                "{}",
                style("Not including the requestor's R-DID in alsoKnownAs").color256(CLI_BLUE)
            );
            None
        }
    } else {
        // No aliasing needed
        None
    };

    println!();
    println!("{}", style("VRC Configuration").color256(CLI_BLUE).bold());
    println!("{}", style("=================").bold().color256(CLI_BLUE));
    println!();

    if let Some(reason) = &request.reason {
        println!(
            "{} {}",
            style("The VRC Request provided the following reason:").color256(CLI_BLUE),
            style(reason).color256(CLI_PURPLE)
        );
    }

    let description: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter a description for this VRC (optional, press Enter to skip):")
        .allow_empty(true)
        .interact_text()
        .unwrap();

    let description = if description.trim().is_empty() {
        println!(
            "{}",
            style("No VRC description included.").color256(CLI_ORANGE)
        );
        None
    } else {
        Some(description.trim().to_string())
    };

    println!();
    println!(
        "{}",
        style(
            "A human-readable name can help others understand the purpose or reason for this VRC."
        )
        .color256(CLI_BLUE)
    );
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Include a human-readable name for this VRC (optional, press Enter to skip):")
        .allow_empty(true)
        .interact_text()
        .unwrap();

    let name = if name.trim().is_empty() {
        println!("{}", style("No VRC name included.").color256(CLI_ORANGE));
        println!();
        None
    } else {
        Some(name.trim().to_string())
    };

    // Set the relationshipType attribute
    let mut items = vec![
        "Do not include a relationship type".to_string(),
        "Set a custom relationship type".to_string(),
    ];

    if let Some(type_) = &request.type_ {
        items.push(["Use the requestor's suggested relationship type: ", type_].concat());
    }

    let relationship_type = match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Include a relationship type for this VRC?")
        .default(0)
        .items(items)
        .interact()?
    {
        0 => None,
        1 => {
            let custom_relationship_type: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter a custom relationship type: ")
                .interact_text()
                .unwrap();
            Some(custom_relationship_type.trim().to_string())
        }
        2 => {
            println!(
                "{}{}",
                style("Using the requestor's suggested relationship type: ").color256(CLI_BLUE),
                style(request.type_.as_deref().unwrap()).color256(CLI_PURPLE)
            );
            Some(request.type_.as_deref().unwrap().to_string())
        }
        _ => None,
    };

    println!(
        "{} {}",
        style("Did the requestor request to include a relationship start date?").color256(CLI_BLUE),
        if request.start_date {
            style("YES").color256(CLI_GREEN)
        } else {
            style("NO").color256(CLI_ORANGE)
        }
    );
    let r_start_date_str = r_created
        .with_timezone(&Local)
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    println!(
        "{} {}",
        style("Relationship start date: ").color256(CLI_BLUE),
        style(&r_start_date_str).color256(CLI_GREEN)
    );
    let start_date = match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Set a start date for the relationship?")
        .item(format!(
            "Use the requestor's suggested start date: {}",
            &r_start_date_str
        ))
        .item("Set start date to now")
        .item("Set custom start date")
        .item("No start date")
        .default(0)
        .interact()?
    {
        0 => Some(r_created),
        1 => Some(Utc::now()),
        2 => {
            println!(
                "{}",
                style("Timestamp format must be in ISO 8601 Format.").color256(CLI_BLUE)
            );
            let custom_start_date: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter custom start date (e.g., 2025-12-01T14:09:29+08:00): ")
                .validate_with(|input: &String| -> Result<(), &str> {
                    if DateTime::parse_from_rfc3339(input).is_ok() {
                        Ok(())
                    } else {
                        Err("Invalid date-time format. Use ISO 8601 format (e.g., 2025-12-01T14:09:29+08:00).")
                    }
                })
                .interact_text()
                .unwrap();
            Some(
                DateTime::parse_from_rfc3339(&custom_start_date)
                    .unwrap()
                    .to_utc(),
            )
        }
        _ => None,
    };

    println!(
        "{} {}",
        style("Did the requestor request to include a relationship end date?").color256(CLI_BLUE),
        if request.end_date {
            style("YES").color256(CLI_GREEN)
        } else {
            style("NO").color256(CLI_ORANGE)
        }
    );
    let end_date = match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Set an end date for the relationship?")
        .item("Set end date to now")
        .item("Set custom end date")
        .item("No end date")
        .default(if request.end_date { 0 } else { 2 })
        .interact()?
    {
        0 => Some(Utc::now()),
        1 => {
            println!(
                "{}",
                style("Timestamp format must be in ISO 8601 Format.").color256(CLI_BLUE)
            );
            let custom_end_date: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter custom end date (e.g., 2025-12-01T14:09:29+08:00): ")
                .validate_with(|input: &String| -> Result<(), &str> {
                    if DateTime::parse_from_rfc3339(input).is_ok() {
                        Ok(())
                    } else {
                        Err("Invalid date-time format. Use ISO 8601 format (e.g., 2025-12-01T14:09:29+08:00).")
                    }
                })
                .interact_text()
                .unwrap();
            Some(
                DateTime::parse_from_rfc3339(&custom_end_date)
                    .unwrap()
                    .to_utc(),
            )
        }
        _ => None,
    };

    let valid_from = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Should the VRC be valid from now?")
        .default(true)
        .interact()?
    {
        Local::now()
    } else {
        let now = Local::now();
        println!(
            "{}",
            style("The timestamp format must be in ISO 8601 Format.").color256(CLI_BLUE)
        );
        let custom_valid_from: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter a valid from date-time for this VRC (e.g., 2025-12-01T14:09:29+08:00): ")
            .default(now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            .validate_with(|input: &String| -> Result<(), &str> {
                if DateTime::parse_from_rfc3339(input).is_ok() {
                    Ok(())
                } else {
                    Err("Invalid date-time format. Use ISO 8601 format (e.g., 2025-12-01T14:09:29+08:00).")
                }
            })
            .interact_text()
            .unwrap();

        custom_valid_from.parse().unwrap()
    };

    let credential_subject = CredentialSubject {
        from: FromSubject::new(
            config.public.persona_did.to_string(),
            their_p_did.to_string(),
            from_name,
            our_also_known_as,
            &tdk.get_shared_state().secrets_resolver,
        )
        .await?,
        to: ToSubject::new(their_p_did.to_string(), their_name, to_also_known_as),
        relationship_type,
        start_date,
        end_date,
        session: None,
    };

    let mut vrc = Vrc {
        issuer: config.public.persona_did.to_string(),
        valid_from: valid_from.to_utc(),
        name,
        description,
        credential_subject,
        ..Default::default()
    };

    let secret = config.get_persona_keys(tdk).await?.signing.secret;

    let proof = DataIntegrityProof::sign_jcs_data(&vrc, None, &secret, None)?;
    vrc.proof = Some(proof);

    // Send VRC to the requestor
    let msg = vrc.message(&our_r_did, &their_r_did, Some(&task_id))?;

    // Pack the message
    let (msg, _) = msg
        .pack_encrypted(
            &their_r_did,
            Some(&our_r_did),
            Some(&our_r_did),
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
        &config.persona_did.profile,
        false,
        &msg,
        None,
        &config.public.mediator_did,
        their_r_did.as_str(),
        None,
        None,
        false,
    )
    .await?;

    println!(
        "{}\n{}",
        style("Issued VRC").color256(CLI_BLUE).underlined().bold(),
        style(serde_json::to_string_pretty(&vrc)?).color256(CLI_WHITE)
    );

    config
        .private
        .vrcs_issued
        .insert(&their_p_did, Rc::new(vrc));

    config.public.logs.insert(
        LogFamily::Task,
        format!(
            "Issued VRC for remote P-DID({}) Task ID({})",
            their_p_did, task_id
        ),
    );

    config.private.tasks.remove(&task_id);

    Ok(true)
}

/// Shows all VRC's on screen
pub fn vrcs_show_all(config: &Config) {
    // Merge the keys from both issued and received VRC's together
    let mut keys: HashSet<Rc<String>> = config.private.vrcs_received.keys().cloned().collect();

    keys.extend(
        config
            .private
            .vrcs_issued
            .keys()
            .cloned()
            .collect::<HashSet<Rc<String>>>(),
    );

    if keys.is_empty() {
        println!(
            "{}{}{}",
            style("No Verifiable Relationship Credentials exist yet... Run ").color256(CLI_ORANGE),
            style("lkmv vrcs request").color256(CLI_WHITE),
            style(" to create a VRC request to someone").color256(CLI_ORANGE)
        );
        return;
    }

    for remote in keys {
        vrcs_show_relationship(&remote, config);
    }
}

/// Shows all VRC's for a relationship
/// remote: Must be the remote DID of the relationship (can be R-DID or P-DID)
pub fn vrcs_show_relationship(remote: &Rc<String>, config: &Config) {
    let relationship: Relationship =
        if let Some(relationship) = config.private.relationships.find_by_remote_did(remote) {
            let guard = relationship.lock().unwrap();
            guard.clone()
        } else {
            println!(
                "{}{}",
                style("ERROR: Missing relationship record for DID: ").color256(CLI_RED),
                style(remote.as_str()).color256(CLI_ORANGE)
            );
            return;
        };

    let Some(contact) = config
        .private
        .contacts
        .find_contact(&relationship.remote_p_did)
    else {
        println!(
            "{}{}",
            style("ERROR: Missing contact record for DID: ").color256(CLI_RED),
            style(&relationship.remote_p_did).color256(CLI_ORANGE)
        );
        return;
    };

    println!();
    print!(
        "{}{} {}{}",
        style("Relationship Alias: ").color256(CLI_BLUE).bold(),
        if let Some(alias) = &contact.alias {
            style(alias.as_str()).color256(CLI_GREEN)
        } else {
            style("<No Alias>").color256(CLI_ORANGE).italic()
        },
        style("Persona DID: ").color256(CLI_BLUE).bold(),
        style(&relationship.remote_p_did).color256(CLI_PURPLE)
    );
    println!();

    println!(
        "{}{}",
        style("<-- ").color256(CLI_BLUE).bold(),
        style("You have issued the following VRC's to this Relationship:")
            .color256(CLI_BLUE)
            .bold()
            .underlined()
    );
    if let Some(vrcs) = config.private.vrcs_issued.get(remote)
        && !vrcs.is_empty()
    {
        for (vrc_id, vrc) in vrcs {
            vrc_show(vrc_id, vrc);
            println!();
        }
    } else {
        println!(
            "\t{}",
            style("You haven't issued any VRC's for this relationship").color256(CLI_ORANGE)
        );
        println!();
    }

    println!(
        "{}{}",
        style("--> ").color256(CLI_BLUE).bold(),
        style("You have received the following VRC's for this Relationship:")
            .color256(CLI_BLUE)
            .bold()
            .underlined()
    );
    if let Some(vrcs) = config.private.vrcs_received.get(remote)
        && !vrcs.is_empty()
    {
        for (vrc_id, vrc) in vrcs {
            vrc_show(vrc_id, vrc);
            println!();
        }
    } else {
        println!(
            "\t{}",
            style("You haven't received any VRC's for this relationship").color256(CLI_ORANGE)
        );
        println!();
    }
}

/// Prints a vrc to the screen
pub fn vrc_show(vrc_id: &str, vrc: &Vrc) {
    println!(
        "\t{}{}",
        style("VRC ID: ").color256(CLI_BLUE).bold(),
        style(vrc_id).color256(CLI_PURPLE)
    );

    print!("\t  {}", style("Name: ").color256(CLI_BLUE).bold());
    if let Some(name) = &vrc.name {
        print!("{}", style(name).color256(CLI_WHITE));
    } else {
        print!("{}", style("N/A").color256(CLI_ORANGE));
    }
    println!();

    print!("\t  {}", style("Description: ").color256(CLI_BLUE).bold());
    if let Some(description) = &vrc.description {
        print!("{}", style(description).color256(CLI_WHITE));
    } else {
        print!("{}", style("N/A").color256(CLI_ORANGE));
    }
    println!();

    if let Some(rel_type) = &vrc.credential_subject.relationship_type {
        println!(
            "\t  {}{}",
            style("Relationship Type: ").color256(CLI_BLUE).bold(),
            style(rel_type).color256(CLI_WHITE)
        );
    }

    println!(
        "\t  {}{} {}{} {}{}",
        style("Valid From: ").color256(CLI_BLUE).bold(),
        style(
            &vrc.valid_from
                .with_timezone(&Local)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        )
        .color256(CLI_WHITE),
        style("Started?: ").color256(CLI_BLUE).bold(),
        if let Some(start_date) = vrc.credential_subject.start_date {
            style(
                start_date
                    .with_timezone(&Local)
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            )
            .color256(CLI_WHITE)
        } else {
            style("N/A".to_string()).color256(CLI_ORANGE)
        },
        style("End Date?: ").color256(CLI_BLUE).bold(),
        if let Some(end_date) = vrc.credential_subject.end_date {
            style(
                end_date
                    .with_timezone(&Local)
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            )
            .color256(CLI_WHITE)
        } else {
            style("N/A".to_string()).color256(CLI_ORANGE)
        },
    );
}

/// Prints a VRC JSON to screen
pub fn show_vrc_by_id(config: &Config, id: &str) {
    if let Some(vrc) = config.vrcs.get(&Rc::new(id.to_string())) {
        println!(
            "{}{}\n{}",
            style("VRC ID: ").color256(CLI_BLUE).bold(),
            style(id).color256(CLI_PURPLE),
            style(serde_json::to_string_pretty(&vrc).unwrap()).color256(CLI_WHITE)
        )
    } else {
        println!(
            "{}{}",
            style("ERROR: No VRC found with ID: ").color256(CLI_RED),
            style(id).color256(CLI_ORANGE)
        )
    }
}
