/*!
*   Handles inbound relationship requests
*/

use std::rc::Rc;

use crate::{CLI_ORANGE, CLI_PURPLE, config::Config, log::LogFamily, relationships::Relationship};
use anyhow::{Result, bail};
use console::style;

impl Relationship {
    /// Accepts an incoming relationship request from a remote party
    pub async fn accept_request() -> Result<()> {
        Ok(())
    }
}

impl Config {
    /// Handles rejection of a relationship request
    pub fn handle_reject_relationship(
        &mut self,
        task_id: &Rc<String>,
        reason: Option<&str>,
    ) -> Result<()> {
        // Remove the relationship entry
        let Some(relationship) = self.private.relationships.remove_by_task_id(task_id) else {
            println!(
                "{}{}{}",
                style("WARN: Couldn't find relationship with task ID(").color256(CLI_ORANGE),
                style(task_id).color256(CLI_PURPLE),
                style(") to reject").color256(CLI_ORANGE)
            );
            bail!("Couldn't find relationship");
        };

        let reason = if let Some(reason) = reason {
            reason.to_string()
        } else {
            "NO REASON PROVIDED".to_string()
        };

        self.public.logs.insert(
            LogFamily::Relationship,
            format!(
                "Removed relationship ({}) request as rejected by remote entity Reason: {}",
                task_id, reason
            ),
        );

        self.private.tasks.remove(task_id);

        self.public.logs.insert(
            LogFamily::Task,
            format!(
                "Relationship request rejected by remote DID({}) Task ID({}) Reason({})",
                relationship.remote_did, task_id, reason
            ),
        );

        Ok(())
    }
}
