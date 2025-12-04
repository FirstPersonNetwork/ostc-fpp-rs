use std::{rc::Rc, sync::Mutex};

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE,
    config::Config,
    log::LogFamily,
    relationships::{RelationshipRequestBody, messages::send_rejection},
    tasks::{Task, TaskType, Tasks},
    vrc::{
        interact::{interact_vrc_inbound, interact_vrc_outbound_request},
        request::interact_vrc_inbound_request,
    },
};
use affinidi_tdk::TDK;
use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

impl Tasks {
    /// Console interaction for this task
    pub async fn interact_task(
        task: &Rc<Mutex<Task>>,
        tdk: &TDK,
        config: &mut Config,
    ) -> Result<bool> {
        let type_ = { task.lock().unwrap().type_.clone() };
        Ok(match type_ {
            TaskType::RelationshipRequestInbound {
                from,
                to: _,
                request,
            } => interact_relationship_request(tdk, config, task, &from, &request).await?,
            TaskType::RelationshipRequestAccepted => {
                interact_relationship_accepted(config, task).await?
            }
            TaskType::VRCRequestInbound {
                request,
                relationship,
            } => interact_vrc_inbound_request(tdk, config, task, &request, &relationship).await?,
            TaskType::VRCRequestOutbound { relationship } => {
                interact_vrc_outbound_request(config, task, &relationship)?
            }
            TaskType::VRCIssued { vrc } => interact_vrc_inbound(config, task, vrc)?,
            _ => {
                // Do nothing
                false
            }
        })
    }
}

/// Handles the menu for an interactive inbound relationship request
async fn interact_relationship_request(
    tdk: &TDK,
    config: &mut Config,
    task: &Rc<Mutex<Task>>,
    from: &Rc<String>,
    request: &RelationshipRequestBody,
) -> Result<bool> {
    let task_id = { task.lock().unwrap().id.clone() };

    // Show relationship request info
    println!();
    println!(
        "{}{} {}{}",
        style("Task ID: ").color256(CLI_BLUE),
        style(&task_id).color256(CLI_PURPLE),
        style("Type: ").color256(CLI_BLUE),
        style("Inbound Relationship Request").color256(CLI_PURPLE)
    );

    println!(
        "{}{}",
        style("From: ").color256(CLI_BLUE),
        style(from).color256(CLI_PURPLE)
    );

    print!(
        "{}",
        style("Rquesting to use random relationship DID?").color256(CLI_BLUE)
    );

    if request.did == from.as_str() {
        print!(" {}", style("NO").color256(CLI_GREEN));
    } else {
        print!(" {}", style("YES").color256(CLI_GREEN).blink());
    }

    if let Some(reason) = &request.reason {
        println!(
            " {}{}",
            style("Reason: ").color256(CLI_BLUE),
            style(reason).color256(CLI_PURPLE)
        );
    } else {
        println!(
            " {}{}",
            style("Reason: ").color256(CLI_BLUE),
            style("No reason provided").color256(CLI_ORANGE)
        );
    }

    println!();

    match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Task Action?")
        .item("Accept this Relationship request")
        .item("Reject this Relationship request")
        .item("Delete this Relationship request (Does not notify the other party)")
        .item("Return to previous menu?")
        .interact()?
    {
        0 => {
            // Accept
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Are you sure you want to accept this Relationship request?")
                .default(true)
                .interact()?
            {
                // Accept the relationship request
                config
                    .handle_relationship_request_send_accept(tdk, from, &task_id, &request.did)
                    .await?;

                task.lock().unwrap().type_ = TaskType::RelationshipRequestAccepted;

                Ok(true)
            } else {
                Ok(false)
            }
        }
        1 => {
            // Reject

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
                .with_prompt("Are you sure you want to reject this Relationship request?")
                .default(true)
                .interact()?
            {
                send_rejection(tdk, config, from, reason.as_deref(), &task_id).await?;

                config.private.tasks.remove(&task_id);
                config.public.logs.insert(
                    LogFamily::Task,
                    format!(
                        "Rejected Relationship request from remote DID({}) Task ID({}) Reason: {}",
                        from,
                        task_id,
                        reason.as_deref().unwrap_or("NO REASON PROVIDED")
                    ),
                );
                Ok(true)
            } else {
                // Cancel rejection
                Ok(false)
            }
        }
        2 => {
            // Delete

            println!("{}", style("When you delete a relationship request, no response is sent to the initiator of the request. Deleting acts as a silent ignore...").color256(CLI_BLUE));
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Are you sure you want to DELETE this Relationship request?")
                .default(false)
                .interact()?
            {
                config.private.tasks.remove(&task_id);
                config.public.logs.insert(
                    LogFamily::Task,
                    format!(
                        "Deleted Relationship request from remote DID({}) Task ID({})",
                        from, task_id
                    ),
                );
                Ok(true)
            } else {
                Ok(false)
            }
        }
        3 => {
            // Return to previous menu
            Ok(false)
        }
        _ => Ok(false),
    }
}

/// Limited interaction for a relationship acceptance that is in progress
async fn interact_relationship_accepted(
    config: &mut Config,
    task: &Rc<Mutex<Task>>,
) -> Result<bool> {
    let task_id = { task.lock().unwrap().id.clone() };

    let relationship =
        if let Some(relationship) = config.private.relationships.find_by_task_id(&task_id) {
            relationship
        } else {
            println!(
                "{}{}",
                style("WARN: Couldn't find relationship for task ID: ").color256(CLI_ORANGE),
                style(&task_id).color256(CLI_PURPLE)
            );

            println!(
                "{}",
                style("Removing this task as it is no longer valid...").color256(CLI_ORANGE)
            );

            config.private.tasks.remove(&task_id);
            return Ok(true);
        };

    let from = { relationship.lock().unwrap().remote_p_did.clone() };
    // Show relationship request info
    println!();
    println!(
        "{}{} {}{}",
        style("Task ID: ").color256(CLI_BLUE),
        style(&task_id).color256(CLI_PURPLE),
        style("Type: ").color256(CLI_BLUE),
        style("Accepted Relationship Request").color256(CLI_PURPLE)
    );

    println!(
        "{}{}",
        style("From: ").color256(CLI_BLUE),
        style(&from).color256(CLI_PURPLE)
    );

    println!();

    match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Task Action?")
        .item("Delete this Relationship request (Does not notify the other party)")
        .item("Return to previous menu?")
        .interact()?
    {
        0 => {
            println!("{}", style("When you delete a relationship request, no response is sent to the initiator of the request. Deleting acts as a silent ignore...").color256(CLI_BLUE));
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Are you sure you want to DELETE this Relationship request?")
                .default(false)
                .interact()?
            {
                config.private.tasks.remove(&task_id);
                config.public.logs.insert(
                    LogFamily::Task,
                    format!(
                        "Deleted Relationship request from remote DID({}) Task ID({})",
                        from, task_id
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
