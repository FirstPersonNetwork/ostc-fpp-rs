use std::rc::Rc;

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_RED,
    config::Config,
    log::LogFamily,
    tasks::{MessageType, TaskType},
};
use affinidi_tdk::{
    TDK,
    messaging::messages::{DeleteMessageRequest, FetchDeletePolicy, fetch::FetchOptions},
};
use anyhow::Result;
use console::style;

/// Fetches tasks from the DIDComm mediator and returns the number of new tasks retrieved
pub async fn fetch_tasks(tdk: &TDK, config: &mut Config) -> Result<u32> {
    let atm = tdk.atm.clone().unwrap();

    let msgs = atm
        .fetch_messages(
            &config.community_did.profile,
            &FetchOptions {
                limit: 100,
                start_id: None,
                delete_policy: FetchDeletePolicy::DoNotDelete,
            },
        )
        .await?;

    println!(
        "{}{}",
        style(msgs.success.len()).color256(CLI_GREEN),
        style(" tasks fetched successfully:").color256(CLI_BLUE)
    );

    let mut task_count: u32 = 0;
    let mut delete_list = DeleteMessageRequest::default();

    for msg in &msgs.success {
        if let Some(message) = &msg.msg {
            let (unpacked_msg, _) = atm.unpack(message).await?;
            // Ensure message is deleted after processing
            delete_list.message_ids.push(msg.msg_id.clone());

            // No anonymous messages are allowed
            let from_did = if let Some(did) = &unpacked_msg.from {
                Rc::new(did.to_string())
            } else {
                // Ignore this TASK as it is anonymous
                println!("{}", style("WARN: An anonymous message has been received. These are not allowed as there is no ability to reply/respond to an anonymous message. Ignoring this message").color256(CLI_ORANGE));
                delete_list.message_ids.push(unpacked_msg.id.clone());
                continue;
            };

            let to_did = if let Some(to) = &unpacked_msg.to {
                if to.contains(&config.public.community_did) {
                    // Message is addressed to us
                    config.public.community_did.clone()
                } else {
                    // Ignore this TASK as it isn't addressed to us
                    println!("{}", style("WARN: An incoming message is not addressed to our Community DID. Ignoring this message for safety.").color256(CLI_ORANGE));
                    println!(
                        "  {}{}",
                        style("from: ").color256(CLI_ORANGE),
                        style(from_did).color256(CLI_PURPLE)
                    );
                    delete_list.message_ids.push(unpacked_msg.id.clone());
                    continue;
                }
            } else {
                // Ignore this TASK as it isn't addressed correctly
                println!("{}", style("WARN: An incoming message is missing the to: address field. This is going to be ignored for safety.").color256(CLI_ORANGE));
                println!(
                    "  {}{}",
                    style("from: ").color256(CLI_ORANGE),
                    style(from_did).color256(CLI_PURPLE)
                );
                delete_list.message_ids.push(unpacked_msg.id.clone());
                continue;
            };

            let (task_type_style, task_type) =
                if let Ok(msg_type) = MessageType::try_from(&unpacked_msg) {
                    match msg_type {
                        MessageType::RelationshipRequest => {
                            let task_type = TaskType::RelationshipRequestInbound {
                                from: from_did.clone(),
                                to: to_did.clone(),
                                request: serde_json::from_value(unpacked_msg.body)?,
                            };
                            config
                                .private
                                .tasks
                                .new_task(&Rc::new(unpacked_msg.id.clone()), task_type.clone());
                            task_count += 1;
                            (
                                style(msg_type.friendly_name()).color256(CLI_GREEN),
                                task_type,
                            )
                        }
                        MessageType::RelationshipRequestRejected => {
                            todo!("Implement rejected message handling")
                        }
                        MessageType::RelationshipRequestAccepted => {
                            todo!("Implement accepted message handling")
                        }
                    }
                } else {
                    println!(
                        "{}{}",
                        style("INVALID Task Type: ").color256(CLI_RED),
                        style(unpacked_msg.type_).color256(CLI_ORANGE)
                    );
                    continue;
                };

            println!(
                "{}{} {}{}",
                style("Added Task Id: ").color256(CLI_BLUE),
                style(&unpacked_msg.id).color256(CLI_PURPLE),
                style("Type: ").color256(CLI_BLUE),
                style(task_type_style).color256(CLI_PURPLE),
            );

            config.public.logs.insert(
                LogFamily::Task,
                &format!(
                    "Fetched: Task ID({}) Type({}) From({}) To({})",
                    &unpacked_msg.id, task_type, from_did, &to_did
                ),
            );
        } else {
            println!(
                "{}",
                style("ERROR: Task fetched, but no message was found!").color256(CLI_RED)
            );
        }
        println!();
    }

    // Delete messages as we have retrieved them
    if !delete_list.message_ids.is_empty() {
        match atm
            .delete_messages_direct(&config.community_did.profile, &delete_list)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                println!(
                    "{}",
                    style(format!(
                        "WARN: Couldn't delete tasks from server. Reason: {}",
                        e
                    ))
                    .color256(CLI_ORANGE)
                );
            }
        }
    }

    Ok(task_count)
}
