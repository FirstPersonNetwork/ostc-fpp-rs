use crate::{CLI_BLUE, CLI_GREEN, CLI_RED, config::Config};
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
            let (unpacked_msg, unpacked_meta) = atm.unpack(&message).await?;

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
