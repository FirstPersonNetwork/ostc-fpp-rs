use crate::{CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_RED, config::Config, tasks::TaskTypes};
use affinidi_tdk::{
    TDK,
    messaging::messages::{FetchDeletePolicy, fetch::FetchOptions},
};
use anyhow::Result;
use console::style;

pub async fn fetch_tasks(tdk: &TDK, config: &Config) -> Result<()> {
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

    for msg in msgs.success {
        if let Some(message) = msg.msg {
            let (unpacked_msg, _) = atm.unpack(&message).await?;

            let task_type_style = if let Ok(task_type) = TaskTypes::try_from(&unpacked_msg) {
                style(task_type.friendly_name()).color256(CLI_GREEN)
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

    Ok(())
}
