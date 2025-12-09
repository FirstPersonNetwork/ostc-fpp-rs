use crate::config::Config;
use affinidi_tdk::TDK;
use anyhow::Result;
use clap::ArgMatches;

pub async fn maintainers_entry(
    tdk: TDK,
    config: &mut Config,
    profile: &str,
    args: &ArgMatches,
) -> Result<()> {
    Ok(())
}
