/*! Main entry point for task management
*   A [Task] is something that requires action on behalf of the user
*/

use crate::{
    CLI_BLUE, CLI_ORANGE, CLI_PURPLE, CLI_RED, config::Config, relationships::Relationship,
    tasks::fetch::fetch_tasks,
};
use affinidi_tdk::{TDK, messaging::profiles::ATMProfile};
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use console::{StyledObject, Term, style};
use dialoguer::{Select, theme::ColorfulTheme};
use lkmv::{
    relationships::RelationshipRequestBody,
    vrc::{Vrc, VrcRequest},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Display,
    rc::Rc,
    sync::{Arc, Mutex},
};

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
        relationship: Rc<Mutex<Relationship>>,
    },
    TrustPong,
    VRCRequestOutbound {
        relationship: Rc<Mutex<Relationship>>,
    },
    VRCRequestInbound {
        request: VrcRequest,
        relationship: Rc<Mutex<Relationship>>,
    },
    VRCRequestRejected,
    VRCIssued {
        vrc: Box<Vrc>,
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
            TaskType::TrustPong => "Trust Pong Received",
            TaskType::VRCRequestOutbound { .. } => "VRC Request Sent",
            TaskType::VRCRequestInbound { .. } => "VRC Request Received",
            TaskType::VRCRequestRejected => "VRC Request Rejected",
            TaskType::VRCIssued { .. } => "VRC Issued",
        };
        write!(f, "{}", friendly_name)
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
                print!(
                    "{}{} {}{} {}{}",
                    style("Id: ").color256(CLI_BLUE),
                    style(&task_id).color256(CLI_PURPLE),
                    style("Type: ").color256(CLI_BLUE),
                    style(&task.type_).color256(CLI_PURPLE),
                    style("Created: ").color256(CLI_BLUE),
                    style(&task.created).color256(CLI_PURPLE),
                );
                match &task.type_ {
                    TaskType::TrustPing { relationship, .. } => {
                        let lock = relationship.lock().unwrap();
                        print!(
                            " {} {}",
                            style("Remote P-DID:").color256(CLI_BLUE),
                            style(&lock.remote_p_did).color256(CLI_PURPLE)
                        );
                    }
                    TaskType::VRCRequestOutbound { relationship } => {
                        let lock = relationship.lock().unwrap();
                        print!(
                            " {} {}",
                            style("Remote P-DID:").color256(CLI_BLUE),
                            style(&lock.remote_p_did).color256(CLI_PURPLE)
                        );
                    }
                    _ => {}
                }
                println!();
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
    pub async fn interact(tdk: &TDK, config: &mut Config, term: &Term) -> Result<bool> {
        let mut change_flag = false; // set to true if config changed
        loop {
            // fetch tasks in case there are new ones
            if fetch_tasks(tdk, config, term, &config.persona_did.profile.clone()).await? > 0 {
                change_flag = true;
            }

            let profiles: Vec<Arc<ATMProfile>> = config.atm_profiles.values().cloned().collect();
            for profile in profiles {
                if fetch_tasks(tdk, config, term, &profile).await? > 0 {
                    change_flag = true;
                }
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
    term: &Term,
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
            let mut change_flag = false;
            if fetch_tasks(&tdk, config, term, &config.persona_did.profile.clone()).await? > 0 {
                change_flag = true;
            }
            let profiles: Vec<Arc<ATMProfile>> = config.atm_profiles.values().cloned().collect();
            for profile in profiles {
                if fetch_tasks(&tdk, config, term, &profile).await? > 0 {
                    change_flag = true;
                }
            }
            if change_flag {
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

            if Tasks::interact(&tdk, config, term).await? {
                config.save(profile)?;
            }
        }
        Some(("clear", sub_args)) => {
            // Removes all tasks from the remote server as well as locally
            let force = sub_args.get_flag("force");
            let remote = sub_args.get_flag("remote");

            if Tasks::clear_all(&tdk, config, force, remote).await? {
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
