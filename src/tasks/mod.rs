/*! Main entry point for task management
*/

use affinidi_tdk::TDK;
use anyhow::{Result, bail};
use clap::ArgMatches;
use console::style;

use crate::{CLI_ORANGE, CLI_RED, tasks::fetch::fetch_tasks};

pub mod fetch;

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
