/* Linux Kernel Maintainer Verification Tool
*
*/

use crate::{
    config::Config,
    setup::{cli_setup, pgp_export::ask_export_community_did_keys},
};
use affinidi_tdk::{TDK, common::config::TDKConfigBuilder};
use anyhow::{Result, bail};
use clap::{Arg, Command};
use console::{Term, style};
use dialoguer::{Password, theme::ColorfulTheme};
use sha2::Digest;
use status::print_status;
use tracing_subscriber::EnvFilter;

mod config;
#[cfg(feature = "openpgp-card")]
mod openpgp_card;
mod setup;
mod status;

// CLI Color codes
const CLI_BLUE: u8 = 69; // Use for general information
const CLI_GREEN: u8 = 34; // Use for Successful text
const CLI_RED: u8 = 9; // Use for Error messages
const CLI_ORANGE: u8 = 214; // Use for cautionary data
const CLI_PURPLE: u8 = 165; // Use for Example data

// Primary Linux Kernel Mediator DID
const LF_PUBLIC_MEDIATOR_DID: &str =
    "did:webvh:QmetnhxzJXTJ9pyXR1BbZ2h6DomY6SB1ZbzFPrjYyaEq9V:fpp.storm.ws:public-mediator";

fn cli() -> Command {
    Command::new("lkmv")
        .about("Linux Kernel Maintainer Verification")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(Command::new("status").about("Displays status of the lkmv tool"))
        .subcommand(Command::new("setup").about("Initial configuration of the lkmv tool"))
        .subcommand(
            Command::new("export")
                .about("Export settings and other information")
                .subcommand(
                    Command::new("pgp-keys").args([
                        Arg::new("passphrase")
                            .short('p')
                            .long("passphrase")
                            .help("Passphrase to lock the exported PGP Secrets with"),
                        Arg::new("user-id")
                            .short('u')
                            .long("user-id")
                            .help("PGP User Id 'name <email_address>' format")
                            .value_name("first_name last_name <email@domain>")
                    ]).about("Exports first set of keys used in your Community DID for Signing, Authentication and Decryption"),
                )
                .arg_required_else_help(true),
        )
}

// Handles initial setup and configuration of the CLI tool
fn initialize(term: &Term) {
    // Setup logging/tracing
    // If no RUST_LOG ENV variable is set, defaults to MAX_LEVEL: ERROR
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    term.set_title("lkmv");
}

#[tokio::main]
async fn main() -> Result<()> {
    let term = Term::stdout();

    initialize(&term);

    match cli().get_matches().subcommand() {
        Some(("status", _)) => {
            let mut tdk = TDK::new(
                TDKConfigBuilder::new()
                    .with_load_environment(false)
                    .build()?,
                None,
            )
            .await?;
            print_status(&term, &mut tdk).await;
        }
        Some(("setup", _)) => match cli_setup(&term) {
            Ok(_) => {
                println!(
                    "\n{}",
                    style("Setup completed successfully.").color256(CLI_GREEN)
                );
            }
            Err(e) => {
                eprintln!("Setup failed: {e}");
            }
        },
        Some(("export", args)) => {
            // Instantiate the TDK
            let mut tdk = TDK::new(
                TDKConfigBuilder::new()
                    .with_load_environment(false)
                    .build()?,
                None,
            )
            .await?;

            let config = match Config::load(&term, &mut tdk).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    println!(
                        "{}{}",
                        style("ERROR: ").color256(CLI_RED),
                        style(e).color256(CLI_ORANGE)
                    );
                    panic!("Exiting...");
                }
            };

            match args.subcommand() {
                Some(("pgp-keys", sub_args)) => {
                    // Export PGP Keys
                    let user_id = sub_args.get_one::<String>("user-id");
                    //ask_export_community_did_keys(&term, &config, user_id);
                }
                _ => {
                    println!(
                        "{} {}",
                        style("ERROR:").color256(CLI_RED),
                        style(
                            "No valid export subcommand was used. Use --help for more information."
                        )
                        .color256(CLI_ORANGE)
                    );
                    bail!("Bad CLI arguments");
                }
            }
        }
        _ => {
            eprintln!("No valid subcommand was used. Use --help for more information.");
        }
    }

    Ok(())
}

/// Prompts user for their unlock code when not using a hardware token
/// returns the SHA256 hash of whatever they entered
pub fn get_unlock_code() -> Result<[u8; 32]> {
    let unlock_code = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Please enter your lkmv unlock code")
        .allow_empty_password(true)
        .interact()
        .unwrap_or_default();

    Ok(sha2::Sha256::digest(unlock_code.as_bytes()).into())
}
