use std::rc::Rc;

use crate::{
    CLI_BLUE, CLI_ORANGE, CLI_PURPLE,
    config::Config,
    log::LogFamily,
    relationships::{RelationshipRequestBody, create_relationship_did, messages::send_rejection},
    tasks::{Task, TaskType},
};
use affinidi_tdk::TDK;
use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

impl Task {
    /// Console interaction for this task
    pub async fn interact(&self, tdk: &TDK, config: &mut Config) -> Result<bool> {
        match &self.type_ {
            TaskType::RelationshipRequestInbound {
                from,
                to: _,
                request,
            } => {
                interact_relationship_request(tdk, config, self, from, request).await?;
            }
            TaskType::RelationshipRequestOutbound => {
                todo!("Implement outbound interaction")
            }
            TaskType::RelationshipRequestRejected => {
                todo!("Implement rejected interaction")
            }
            TaskType::RelationshipRequestAccepted => {
                todo!("Implement accepted interaction")
            }
        }

        Ok(true)
    }
}

/// Handles the menu for an interactive inbound relationship request
async fn interact_relationship_request(
    tdk: &TDK,
    config: &mut Config,
    task: &Task,
    from: &Rc<String>,
    request: &RelationshipRequestBody,
) -> Result<bool> {
    // Show relationship request info
    println!();
    println!(
        "{}{} {}{}",
        style("Task ID: ").color256(CLI_BLUE),
        style(&task.id).color256(CLI_PURPLE),
        style("Type: ").color256(CLI_BLUE),
        style("Inbound Relationship Request").color256(CLI_PURPLE)
    );

    println!(
        "{}{}",
        style("From: ").color256(CLI_BLUE),
        style(from).color256(CLI_PURPLE)
    );

    if let Some(reason) = &request.reason {
        println!(
            "{}{}",
            style("Reason: ").color256(CLI_BLUE),
            style(reason).color256(CLI_PURPLE)
        );
    } else {
        println!(
            "{}{}",
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
                    .handle_relationship_request_accept(tdk, from, &task.id)
                    .await?;

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
                send_rejection(tdk, config, from, reason.as_deref(), &task.id).await?;

                config.private.tasks.remove(&task.id);
                config.public.logs.insert(
                    LogFamily::Task,
                    format!(
                        "Rejected Relationship request from remote DID({}) Task ID({}) Reason: {}",
                        from,
                        task.id,
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
                config.private.tasks.remove(&task.id);
                config.public.logs.insert(
                    LogFamily::Task,
                    format!(
                        "Deleted Relationship request from remote DID({}) Task ID({})",
                        from, task.id
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
