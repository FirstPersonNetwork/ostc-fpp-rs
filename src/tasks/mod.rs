/*! Main entry point for task management
*   A [Task] is something that requires action on behalf of the user
*/

use std::{collections::HashMap, fmt::Display};

use crate::{CLI_BLUE, CLI_ORANGE, CLI_PURPLE, CLI_RED, tasks::fetch::fetch_tasks};
use affinidi_tdk::{TDK, didcomm::Message};
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use console::style;
use serde::{Deserialize, Serialize};

pub mod fetch;

/// Defined Task Types for LKMV
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TaskType {
    RelationshipRequestOutbound,
    RelationshipRequestInbound,
}

impl Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let friendly_name = match self {
            TaskType::RelationshipRequestOutbound => "Relationship Request (Outbound)",
            TaskType::RelationshipRequestInbound => "Relationship Request (Inbound)",
        };
        write!(f, "{}", friendly_name)
    }
}

/// Defined Message Types for LKMV
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MessageType {
    RelationshipRequest,
}

impl MessageType {
    fn friendly_name(&self) -> String {
        match self {
            MessageType::RelationshipRequest => "Relationship Request",
        }
        .to_string()
    }
}

/// Convert TaskTypes to type string
impl From<MessageType> for String {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::RelationshipRequest => {
                "https://linuxfoundation.org/lkmv/1.0/relationship-request".to_string()
            }
        }
    }
}

/// Convert &str to a MessageType based on type URL
impl TryFrom<&str> for MessageType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "https://linuxfoundation.org/lkmv/1.0/relationship-request" => {
                Ok(MessageType::RelationshipRequest)
            }
            _ => bail!("Invalid Task Type: {}", value),
        }
    }
}

/// Convert a DIDComm message to a MessageType
impl TryFrom<&Message> for MessageType {
    type Error = anyhow::Error;

    fn try_from(value: &Message) -> Result<Self> {
        value.type_.as_str().try_into()
    }
}

// ****************************************************************************
// Tasks Struct
// ****************************************************************************

/// Known Tasks that are in progress
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tasks {
    /// key: Task ID
    tasks: HashMap<String, Task>,
}

/// LKMV Task
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Task {
    /// Type of Task
    pub type_: TaskType,

    /// When was this task created?
    pub created: DateTime<Utc>,
}

impl Tasks {
    /// Prints known tasks to the console
    pub fn print_tasks(&self) {
        if self.tasks.is_empty() {
            println!(
                "{}",
                style("There are no tasks currently").color256(CLI_ORANGE)
            );
        } else {
            for (task_id, task) in &self.tasks {
                println!(
                    "{}{} {}{} {}{}",
                    style("Id: ").color256(CLI_BLUE),
                    style(&task_id).color256(CLI_PURPLE),
                    style("Type: ").color256(CLI_BLUE),
                    style(&task.type_).color256(CLI_PURPLE),
                    style("Created: ").color256(CLI_BLUE),
                    style(&task.created).color256(CLI_PURPLE),
                );
            }
        }
    }

    /// Creates and adds a new Task to list of tasks
    pub fn new_task(&mut self, id: &str, type_: TaskType) -> &Task {
        self.tasks.insert(
            id.to_string(),
            Task {
                type_,
                created: Utc::now(),
            },
        );

        self.tasks.get(id).unwrap()
    }

    /// Removes a task by ID
    pub fn remove(&mut self, id: &str) -> bool {
        self.tasks.remove(id).is_some()
    }
}

// ****************************************************************************
// Primary entry point for Tasks from the CLI
// ****************************************************************************

/// Primary entry point for the Tasks module from the CLI
pub async fn tasks_entry(
    tdk: TDK,
    config: &mut crate::config::Config,
    profile: &str,
    args: &ArgMatches,
) -> Result<()> {
    match args.subcommand() {
        Some(("list", _)) => {
            config.private.tasks.print_tasks();
        }
        Some(("remove", sub_args)) => {
            let id = if let Some(id) = sub_args.get_one::<String>("id") {
                id.to_string()
            } else {
                println!(
                    "{}",
                    style("ERROR: A task ID must be specified!").color256(CLI_RED)
                );
                bail!("Invalid CLI options");
            };

            if config.private.tasks.remove(&id) {
                config.save(profile)?;
            }
        }
        Some(("fetch", _)) => {
            if fetch_tasks(&tdk, config).await? > 0 {
                config.save(profile)?;
            }
        }
        _ => {
            println!(
                "{} {}",
                style("ERROR:").color256(CLI_RED),
                style("No valid tasks subcommand was used. Use --help for more information.")
                    .color256(CLI_ORANGE)
            );
            bail!("Invalid CLI Options");
        }
    }

    Ok(())
}
