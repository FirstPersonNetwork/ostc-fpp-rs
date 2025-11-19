/*! Main entry point for task management
*   A [Task] is something that requires action on behalf of the user
*/

use crate::{
    CLI_BLUE, CLI_ORANGE, CLI_PURPLE, CLI_RED, config::Config,
    relationships::RelationshipRequestBody, tasks::fetch::fetch_tasks,
};
use affinidi_tdk::{TDK, didcomm::Message};
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use console::{StyledObject, style};
use dialoguer::{Select, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, rc::Rc, sync::Mutex};

pub mod clear;
pub mod fetch;
pub mod interact;

/// Defined Task Types for LKMV
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TaskType {
    RelationshipRequestOutbound,
    RelationshipRequestInbound {
        from: Rc<String>,
        to: Rc<String>,
        request: RelationshipRequestBody,
    },
    RelationshipRequestRejected,
    RelationshipRequestAccepted,
    RelationshipRequestFinalized,
    TrustPing {
        from: Rc<String>,
        to: Rc<String>,
    },
}

impl Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let friendly_name = match self {
            TaskType::RelationshipRequestOutbound => "Relationship Request (Outbound)",
            TaskType::RelationshipRequestInbound { .. } => "Relationship Request (Inbound)",
            TaskType::RelationshipRequestRejected => "Relationship Request Rejected",
            TaskType::RelationshipRequestAccepted => "Relationship Request Accepted",
            TaskType::RelationshipRequestFinalized => "Relationship Request Finalized",
            TaskType::TrustPing { .. } => "Trust Ping Sent",
        };
        write!(f, "{}", friendly_name)
    }
}

/// Defined Message Types for LKMV
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MessageType {
    RelationshipRequest,
    RelationshipRequestRejected,
    RelationshipRequestAccepted,
    RelationshipRequestFinalize,
    TrustPing,
    TrustPong,
}

impl MessageType {
    fn friendly_name(&self) -> String {
        match self {
            MessageType::RelationshipRequest => "Relationship Request",
            MessageType::RelationshipRequestRejected => "Relationship Request Rejected",
            MessageType::RelationshipRequestAccepted => "Relationship Request Accepted",
            MessageType::RelationshipRequestFinalize => "Relationship Request Finalize",
            MessageType::TrustPing => "Trust Ping (Send)",
            MessageType::TrustPong => "Trust Pong (Receive)",
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
            MessageType::RelationshipRequestRejected => {
                "https://linuxfoundation.org/lkmv/1.0/relationship-request-reject".to_string()
            }
            MessageType::RelationshipRequestAccepted => {
                "https://linuxfoundation.org/lkmv/1.0/relationship-request-accept".to_string()
            }
            MessageType::RelationshipRequestFinalize => {
                "https://linuxfoundation.org/lkmv/1.0/relationship-request-finalize".to_string()
            }
            MessageType::TrustPing => "https://didcomm.org/trust-ping/2.0/ping".to_string(),
            MessageType::TrustPong => {
                "https://didcomm.org/trust-ping/2.0/ping-response".to_string()
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
            "https://linuxfoundation.org/lkmv/1.0/relationship-request-reject" => {
                Ok(MessageType::RelationshipRequestRejected)
            }
            "https://linuxfoundation.org/lkmv/1.0/relationship-request-accept" => {
                Ok(MessageType::RelationshipRequestAccepted)
            }
            "https://linuxfoundation.org/lkmv/1.0/relationship-request-finalize" => {
                Ok(MessageType::RelationshipRequestFinalize)
            }
            "https://didcomm.org/trust-ping/2.0/ping" => Ok(MessageType::TrustPing),
            "https://didcomm.org/trust-ping/2.0/ping-response" => Ok(MessageType::TrustPong),
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
    tasks: HashMap<Rc<String>, Rc<Mutex<Task>>>,
}

/// LKMV Task
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Task {
    /// ID of task
    pub id: Rc<String>,

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
                let task = task.lock().unwrap();
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
    pub fn new_task(&mut self, id: &Rc<String>, type_: TaskType) -> Rc<Mutex<Task>> {
        let task = Rc::new(Mutex::new(Task {
            id: id.clone(),
            type_,
            created: Utc::now(),
        }));
        self.tasks.insert(id.clone(), task.clone());
        task
    }

    /// Removes a task by ID
    pub fn remove(&mut self, id: &Rc<String>) -> bool {
        self.tasks.remove(id).is_some()
    }

    /// Returns task at position pos
    /// Be careful with this, as insertions/removals can change operation
    pub fn get_by_pos(&self, pos: usize) -> Option<Rc<Mutex<Task>>> {
        self.tasks.iter().nth(pos).map(|(_, task)| task.clone())
    }

    /// Retrieves a task by ID or returns None
    pub fn get_by_id(&self, id: &Rc<String>) -> Option<&Rc<Mutex<Task>>> {
        self.tasks.get(id)
    }

    /// Clears all tasks
    /// Returns true if any tasks were removed
    /// Returns false if no changes were made
    pub fn clear(&mut self) -> bool {
        let flag = !self.tasks.is_empty();
        self.tasks.clear();
        flag
    }

    /// Interactive console for handling tasks
    /// Returns true if changes were made to config
    pub async fn interact(tdk: &TDK, config: &mut Config) -> Result<bool> {
        let mut change_flag = false; // set to true if config changed
        loop {
            // fetch tasks in case there are new ones
            if fetch_tasks(tdk, config).await? > 0 {
                change_flag = true;
            }

            if config.private.tasks.tasks.is_empty() {
                println!(
                    "{}",
                    style("There are no tasks to interact with").color256(CLI_ORANGE)
                );
                break;
            }

            let mut select_list: Vec<StyledObject<String>> = config
                .private
                .tasks
                .tasks
                .iter()
                .map(|(id, task)| {
                    style(format!("{} Type: {}", id, task.lock().unwrap().type_))
                        .color256(CLI_PURPLE)
                })
                .collect();
            select_list.push(style("Exit Task Interaction".to_string()).color256(CLI_ORANGE));

            let selected = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select a task to interact with")
                .items(&select_list)
                .default(0)
                .interact()
                .unwrap();

            if selected == select_list.len() - 1 {
                // exit option
                break;
            } else if let Some(task) = config.private.tasks.get_by_pos(selected) {
                if Tasks::interact_task(&task, tdk, config).await? {
                    change_flag = true;
                }
            } else {
                println!(
                    "{}",
                    style("WARN: No valid task selected!").color256(CLI_ORANGE)
                );
            }
        }

        Ok(change_flag)
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

            if config.private.tasks.remove(&Rc::new(id)) {
                config.save(profile)?;
            }
        }
        Some(("fetch", _)) => {
            if fetch_tasks(&tdk, config).await? > 0 {
                config.save(profile)?;
            }
        }
        Some(("interact", sub_args)) => {
            if let Some(task_id) = sub_args.get_one::<String>("id").map(|id| id.to_string()) {
                let task =
                    if let Some(task) = config.private.tasks.get_by_id(&Rc::new(task_id.clone())) {
                        task.clone()
                    } else {
                        println!(
                            "{}{}",
                            style("ERROR: No task with ID: ").color256(CLI_RED),
                            style(task_id).color256(CLI_ORANGE)
                        );
                        bail!("Unknown Task ID");
                    };

                if Tasks::interact_task(&task, &tdk, config).await? {
                    config.save(profile)?;
                    return Ok(());
                }
            }

            if Tasks::interact(&tdk, config).await? {
                config.save(profile)?;
            }
        }
        Some(("clear", sub_args)) => {
            // Removes all tasks from the remote server as well as locally
            let force = sub_args.get_flag("force");

            if Tasks::clear_all(&tdk, config, force).await? {
                config.save(profile)?;
                return Ok(());
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
