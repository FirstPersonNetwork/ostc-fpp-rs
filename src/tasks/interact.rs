use std::rc::Rc;

use crate::{
    CLI_BLUE, CLI_ORANGE, CLI_PURPLE,
    config::Config,
    relationships::RelationshipRequestBody,
    tasks::{Task, TaskType},
};
use affinidi_tdk::TDK;
use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Select, theme::ColorfulTheme};

impl Task {
    /// Console interaction for this task
    pub async fn interact(&self, tdk: &TDK, config: &mut Config) -> Result<bool> {
        match &self.type_ {
            TaskType::RelationshipRequestInbound { from, to, request } => {
                interact_relationship_request(config, &self, from, request).await?;
            }
            TaskType::RelationshipRequestOutbound => {
                todo!("Implement outbound interaction")
            }
            TaskType::RelationshipRequestRejected => {
                todo!("Implement rejected interaction")
            }
        }

        Ok(true)
    }
}

/// Handles the menu for an interactive inbound relationship request
async fn interact_relationship_request(
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
            Ok(true)
        }
        1 => {
            // Reject
            Ok(true)
        }
        2 => {
            // Delete
            //
            Ok(true)
        }
        3 => {
            // Return to previous menu
            Ok(false)
        }
        _ => Ok(false),
    }
}
