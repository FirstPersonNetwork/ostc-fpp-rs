use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_RED,
    config::Config,
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

            let task_type_style = if let Ok(msg_type) = MessageType::try_from(&unpacked_msg) {
                match msg_type {
                    MessageType::RelationshipRequest => {
                        config
                            .private
                            .tasks
                            .new_task(&unpacked_msg.id, TaskType::RelationshipRequestInbound);
                        task_count += 1;
                        style(msg_type.friendly_name()).color256(CLI_GREEN)
                    }
                }
            } else {
                style(format!("INVALID Task Type: {}", unpacked_msg.type_)).color256(CLI_ORANGE)
            };

            println!(
                "{}{}",
                style("Task Type: ").color256(CLI_BLUE),
                task_type_style
            );

            println!("Task: {unpacked_msg:#?}");
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
