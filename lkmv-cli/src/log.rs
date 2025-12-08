/*!
*  Manages a log of messages that can be helpfuil to see what has happened in the past.
*/

use crate::{CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE};
use chrono::Utc;
use console::style;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt::Display};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum LogFamily {
    Relationship,
    Contact,
    Task,
    Config,
}

impl Display for LogFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LogFamily::Relationship => "RELATIONSHIP",
            LogFamily::Contact => "CONTACT",
            LogFamily::Task => "TASK",
            LogFamily::Config => "CONFIG",
        };
        write!(f, "{}", s)
    }
}

/// Log Messages
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogMessage {
    // When the log message was created
    pub created: chrono::DateTime<Utc>,

    // What type of log is this related to?
    pub type_: LogFamily,

    // Log Message
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Logs {
    pub messages: VecDeque<LogMessage>,
    /// Max number of entries to keep
    pub limit: usize,
}

impl Default for Logs {
    fn default() -> Self {
        Self {
            messages: VecDeque::new(),
            limit: 100,
        }
    }
}

impl Logs {
    /// Insert a new log entry message to the log
    /// Handles keeping the log messages within the limit
    pub fn insert(&mut self, type_: LogFamily, message: String) {
        self.messages.push_back(LogMessage {
            created: Utc::now(),
            type_,
            message,
        });

        if self.messages.len() > self.limit {
            self.messages.pop_front();
        }
    }

    /// Shows all log files to STDOUT
    pub fn show_all(&self) {
        if self.messages.is_empty() {
            println!("{}", style("There are no log entries").color256(CLI_ORANGE));
        } else {
            for log in &self.messages {
                println!(
                    "{} {} {} {}",
                    style(log.created).color256(CLI_GREEN),
                    style(&log.type_).color256(CLI_PURPLE),
                    style("::").color256(CLI_BLUE),
                    style(&log.message).color256(CLI_GREEN)
                );
            }
        }
    }
}
