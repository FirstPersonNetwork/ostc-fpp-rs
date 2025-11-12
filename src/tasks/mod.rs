/*! Main entry point for task management
*/

use crate::{CLI_ORANGE, CLI_RED, tasks::fetch::fetch_tasks};
use affinidi_tdk::{TDK, didcomm::Message};
use anyhow::{Result, bail};
use clap::ArgMatches;
use console::style;

pub mod fetch;

/// Defined Task Types for LKMV
#[derive(Debug)]
#[non_exhaustive]
pub enum TaskTypes {
    RelationshipRequest,
}

impl TaskTypes {
    fn friendly_name(&self) -> String {
        match self {
            TaskTypes::RelationshipRequest => "Relationship Request",
        }
        .to_string()
    }
}

/// Convert TaskTypes to type string
impl From<TaskTypes> for String {
    fn from(value: TaskTypes) -> Self {
        match value {
            TaskTypes::RelationshipRequest => {
                "https://linuxfoundation.org/lkmv/1.0/relationship-request".to_string()
            }
        }
    }
}

/// Convert &str to a TaskType based on type URL
impl TryFrom<&str> for TaskTypes {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "https://linuxfoundation.org/lkmv/1.0/relationship-request" => {
                Ok(TaskTypes::RelationshipRequest)
            }
            _ => bail!("Invalid Task Type: {}", value),
        }
    }
}

/// Convert a DIDComm message to a TaskType
impl TryFrom<&Message> for TaskTypes {
    type Error = anyhow::Error;

    fn try_from(value: &Message) -> Result<Self> {
        value.type_.as_str().try_into()
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
        Some(("fetch", sub_args)) => {
            fetch_tasks(&tdk, config).await?;
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
