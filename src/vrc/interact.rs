/*!
*    Verifiable Relationship Credential Entry Point
*/

use crate::{CLI_ORANGE, CLI_RED, config::Config};
use affinidi_tdk::TDK;
use anyhow::{Result, bail};
use clap::ArgMatches;
use console::style;

/// Primary entry point for VRCs interactions
pub async fn vrcs_entry(
    tdk: TDK,
    config: &mut Config,
    profile: &str,
    args: &ArgMatches,
) -> Result<()> {
    match args.subcommand() {
        Some(("request", sub_args)) => {}
        _ => {
            println!(
                "{} {}",
                style("ERROR:").color256(CLI_RED),
                style("No vrcs tasks subcommand was used. Use --help for more information.")
                    .color256(CLI_ORANGE)
            );
            bail!("Invalid CLI Options");
        }
    }

    Ok(())
}
