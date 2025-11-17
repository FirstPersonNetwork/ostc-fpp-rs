/*!
*   Handles inbound relationship requests
*/

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE,
    config::Config,
    log::LogFamily,
    relationships::{
        Relationship, RelationshipAcceptBody, RelationshipFinalizeBody, RelationshipState,
        create_relationship_did,
    },
};
use affinidi_tdk::{
    TDK,
    didcomm::{Message, PackEncryptedOptions},
};
use anyhow::{Result, bail};
use chrono::Utc;
use console::style;
use dialoguer::{Confirm, Input, theme::ColorfulTheme};
use serde_json::json;
use std::{rc::Rc, sync::Mutex, time::SystemTime};
use uuid::Uuid;

impl Config {
    /// Accepts an incoming relationship request from a remote party and sends the acceptance
    /// message back to them
    pub async fn handle_relationship_request_send_accept(
        &mut self,
        tdk: &TDK,
        from: &Rc<String>,
        task_id: &Rc<String>,
    ) -> Result<()> {
        // What r-did to use for this relationship?
        let r_did = if Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Do you want to create a random relationship DID to be used with this Relationship?")
                    .default(false)
                    .interact()?
        {
            let mediator = self.public.mediator_did.clone(); // Clone so we can borrow config
                // as mutable below
            let r_did = Rc::new(create_relationship_did(tdk, self, &mediator).await?);
            println!(
                "{}{}{}{}",
                style("Generated new Relationship DID for contact ").color256(CLI_GREEN),
                style(from).color256(CLI_PURPLE),
                style(" :: ").color256(CLI_GREEN),
                style(&r_did).color256(CLI_PURPLE)
            );

            self.public.logs.insert(LogFamily::Relationship, format!("Created new r-did ({}) for relationhip from ({}) task ID ({})", r_did, from, task_id));
            r_did
        } else {
            self.public.community_did.clone()
        };

        // Contact Management
        if self.private.contacts.find_contact(from).is_none() {
            let alias: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter an alias for this contact (Leave BLANK for no alias)")
                .allow_empty(true)
                .interact_text()?;
            let alias = if alias.trim().is_empty() {
                None
            } else {
                Some(alias.trim().to_string())
            };

            self.private
                .contacts
                .add_contact(tdk, from, alias, false, &mut self.public.logs)
                .await?;
        }

        // Create the DIDComm message
        let msg = create_message_accepted(&self.public.community_did, from, &r_did, task_id)?;

        let atm = tdk.atm.clone().unwrap();

        // Pack the message
        let (msg, _) = msg
            .pack_encrypted(
                from,
                Some(&self.public.community_did),
                Some(&self.public.community_did),
                tdk.did_resolver(),
                &tdk.get_shared_state().secrets_resolver,
                &PackEncryptedOptions {
                    forward: false,
                    ..Default::default()
                },
            )
            .await?;

        atm.forward_and_send_message(
            &self.community_did.profile,
            false,
            &msg,
            None,
            &self.public.mediator_did,
            from,
            None,
            None,
            false,
        )
        .await?;

        println!();
        println!(
            "{}{}",
            style("✅ Succesfully sent Relationship Request Acceptance to ").color256(CLI_GREEN),
            style(from).color256(CLI_PURPLE)
        );

        self.private.relationships.relationships.insert(
            from.clone(),
            Rc::new(Mutex::new(Relationship {
                task_id: task_id.clone(),
                remote_did: from.clone(),
                remote_c_did: from.clone(),
                our_did: r_did.clone(),
                created: Utc::now(),
                state: RelationshipState::RequestAccepted,
            })),
        );

        self.public.logs.insert(
            LogFamily::Relationship,
            format!(
                "Relationship request accepted: remote DID({}) Task ID({})",
                from, task_id
            ),
        );

        Ok(())
    }

    /// Handles rejection of a relationship request
    pub fn handle_relationship_reject(
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
                relationship.lock().unwrap().remote_did,
                task_id,
                reason
            ),
        );

        Ok(())
    }

    /// Handles the inbound accept message from a remote party, this triggers the finalize
    /// relationship establishment message
    pub async fn handle_relationship_inbound_accept(
        &mut self,
        tdk: &TDK,
        from: &Rc<String>,
        task_id: &Rc<String>,
        r_did: &str,
    ) -> Result<()> {
        // Update the relationship state with new r-did if required
        let our_r_did = if let Some(relationship) = self.private.relationships.get(from) {
            let mut lock = relationship.lock().unwrap();
            lock.state = RelationshipState::Established;
            if lock.remote_did.as_str() != r_did {
                lock.remote_did = Rc::new(r_did.to_string());
                self.public.logs.insert(
                    LogFamily::Relationship,
                    format!(
                        "Changing remote DID to a r-did of ({}) for c-did ({}) task ID ({})",
                        r_did, from, task_id
                    ),
                );
            }
            lock.our_did.clone()
        } else {
            println!(
                "{}",
                style(
                    "WARN: Couldn't find relationship for this inbound Relationship accept message!"
                )
                .color256(CLI_ORANGE)
            );
            bail!("Couldn't find relationship for task ID ({})", task_id);
        };

        // Create the DIDComm message
        let msg = create_message_finalize(&self.public.community_did, from, &our_r_did, task_id)?;

        let atm = tdk.atm.clone().unwrap();

        // Pack the message
        let (msg, _) = msg
            .pack_encrypted(
                from,
                Some(&self.public.community_did),
                Some(&self.public.community_did),
                tdk.did_resolver(),
                &tdk.get_shared_state().secrets_resolver,
                &PackEncryptedOptions {
                    forward: false,
                    ..Default::default()
                },
            )
            .await?;

        atm.forward_and_send_message(
            &self.community_did.profile,
            false,
            &msg,
            None,
            &self.public.mediator_did,
            from,
            None,
            None,
            false,
        )
        .await?;

        println!();
        println!(
            "{}{}",
            style("✅ Succesfully sent Relationship Request Finalize to ").color256(CLI_GREEN),
            style(from).color256(CLI_PURPLE)
        );

        self.private.tasks.remove(task_id);

        self.public.logs.insert(
            LogFamily::Relationship,
            format!(
                "Relationship request finalized: remote DID({}) Task ID({})",
                from, task_id
            ),
        );

        Ok(())
    }

    /// Handles the last message of the relationship establishment process
    pub async fn handle_relationship_inbound_finalize(
        &mut self,
        from: &Rc<String>,
        task_id: &Rc<String>,
        r_did: &str,
    ) -> Result<()> {
        // Update the relationship state with new remote r-did if required
        let relationship = if let Some(relationship) = self.private.relationships.get(from) {
            let mut lock = relationship.lock().unwrap();
            lock.state = RelationshipState::Established;
            if lock.remote_did.as_str() != r_did {
                lock.remote_did = Rc::new(r_did.to_string());
                self.public.logs.insert(
                    LogFamily::Relationship,
                    format!(
                        "Changing remote DID to a r-did of ({}) for c-did ({}) task ID ({})",
                        r_did, from, task_id
                    ),
                );
            }
            relationship.clone()
        } else {
            println!(
                "{}",
                style(
                    "WARN: Couldn't find relationship for this inbound Relationship accept message!"
                )
                .color256(CLI_ORANGE)
            );
            bail!("Couldn't find relationship for task ID ({})", task_id);
        };

        println!();
        println!(
            "{}{}",
            style("✅ Relationship successfully established ").color256(CLI_GREEN),
            style(from).color256(CLI_PURPLE)
        );

        let lock = relationship.lock().unwrap();
        print!(
            "  {}{}{}",
            style("Remote: c-did(").color256(CLI_BLUE),
            style(&lock.remote_c_did).color256(CLI_GREEN),
            style(")").color256(CLI_BLUE)
        );
        if lock.remote_c_did == lock.remote_did {
            println!(
                " {}{}{}",
                style("r-did(").color256(CLI_BLUE),
                style("SAME").color256(CLI_GREEN),
                style(")").color256(CLI_BLUE)
            );
        } else {
            println!(
                " {}{}{}",
                style("r-did(").color256(CLI_BLUE),
                style(&lock.remote_did).color256(CLI_PURPLE),
                style(")").color256(CLI_BLUE)
            );
        }

        print!(
            "  {}{}{}",
            style("Local: c-did(").color256(CLI_BLUE),
            style(&self.public.community_did).color256(CLI_GREEN),
            style(")").color256(CLI_BLUE)
        );
        if lock.our_did == self.public.community_did {
            println!(
                " {}{}{}",
                style("r-did(").color256(CLI_BLUE),
                style("SAME").color256(CLI_GREEN),
                style(")").color256(CLI_BLUE)
            );
        } else {
            println!(
                " {}{}{}",
                style("r-did(").color256(CLI_BLUE),
                style(&lock.our_did).color256(CLI_PURPLE),
                style(")").color256(CLI_BLUE)
            );
        }

        self.public.logs.insert(
            LogFamily::Relationship,
            format!(
                "Relationship request finalized: remote DID({}) Task ID({})",
                from, task_id
            ),
        );

        Ok(())
    }
}

/// DIDComm message for when a relationship request has been accepted
fn create_message_accepted(
    from: &str,
    to: &str,
    r_did: &Rc<String>,
    task_id: &Rc<String>,
) -> Result<Message> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let message = Message::build(
        Uuid::new_v4().into(),
        "https://linuxfoundation.org/lkmv/1.0/relationship-request-accept".to_string(),
        json!(RelationshipAcceptBody {
            did: r_did.to_string()
        }),
    )
    .from(from.to_string())
    .to(to.to_string())
    .thid(task_id.to_string())
    .created_time(now)
    .expires_time(60 * 60 * 48) // 48 hours
    .finalize();

    Ok(message)
}

/// DIDComm final message for when a relationship request has been accepted by all parties
fn create_message_finalize(
    from: &str,
    to: &str,
    r_did: &Rc<String>,
    task_id: &Rc<String>,
) -> Result<Message> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let message = Message::build(
        Uuid::new_v4().into(),
        "https://linuxfoundation.org/lkmv/1.0/relationship-request-finalize".to_string(),
        json!(RelationshipFinalizeBody {
            did: r_did.to_string()
        }),
    )
    .from(from.to_string())
    .to(to.to_string())
    .thid(task_id.to_string())
    .created_time(now)
    .expires_time(60 * 60 * 48) // 48 hours
    .finalize();

    Ok(message)
}
