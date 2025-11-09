/* Linux Kernel Maintainer Verification Tool
*
*/

use crate::{
    config::Config,
    relationships::relationships_entry,
    setup::{cli_setup, pgp_export::ask_export_community_did_keys},
};
use affinidi_tdk::{TDK, common::config::TDKConfigBuilder};
use anyhow::{Result, bail};
use clap::{Arg, ArgAction, Command};
use console::{Term, style};
use dialoguer::{Password, theme::ColorfulTheme};
use secrecy::SecretString;
use sha2::Digest;
use status::print_status;
use std::{env, fs, process, str::FromStr};
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
use tracing_subscriber::EnvFilter;

mod config;
mod contacts;
mod messaging;
#[cfg(feature = "openpgp-card")]
mod openpgp_card;
mod relationships;
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
    // To help with readability, some sub-commands get pulled out separately

    // Handles exporting various settings and information
    let export_subcommand = Command::new("export")
        .about("Export settings and other information")
        .subcommands([
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
            ])
            .about("Exports first set of keys used in your Community DID for Signing, Authentication and Decryption"),
            Command::new("settings").args([
                Arg::new("passphrase")
                    .short('p')
                    .long("passphrase")
                    .help("Passphrase to lock the exported settings with"),
                Arg::new("file").short('f').long("file").help("File to save settings to").default_value("export.lkmv"),
            ]).about("Exports settings which can be imported into another lkmv installation")
        ])
        .arg_required_else_help(true);

    // Contact management
    let contacts_subcommand = Command::new("contacts")
        .about("Manage known contacts")
        .subcommand(
            Command::new("add")
                .args([
                    Arg::new("did")
                        .short('d')
                        .long("did")
                        .help("DID of the contact to add")
                        .required(true),
                    Arg::new("alias")
                        .short('a')
                        .long("alias")
                        .help("Optional alias for the contact"),
                    Arg::new("skip")
                        .short('s')
                        .long("skip")
                        .default_value("true")
                        .action(ArgAction::SetFalse)
                        .help("Skip DID Checks"),
                ])
                .about("Add a new DID Contact (Will replace an existing contact if it exists)")
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove an existing DID Contact")
                .group(
                    clap::ArgGroup::new("remove-by")
                        .args(["did", "alias"])
                        .required(true)
                        .multiple(false),
                )
                .args([
                    Arg::new("did")
                        .short('d')
                        .long("did")
                        .help("DID of the contact to remove")
                        .required(true),
                    Arg::new("alias")
                        .short('a')
                        .long("alias")
                        .help("alias for the contact to remove"),
                ])
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("list").about("Lists all known contacts"))
        .arg_required_else_help(true);

    // Relationship management
    let relationships_subcommand = Command::new("relationships")
        .about("Manage relationships")
        .subcommand(
            Command::new("request")
                .args([
                    Arg::new("respondent")
                        .short('d')
                        .long("respondent")
                        .help("Contact alias or DID of the respondent to this relationship request")
                        .required(true),
                    Arg::new("alias")
                        .short('a')
                        .long("alias")
                        .help("Optional alias for the respondent DID"),
                    Arg::new("reason")
                        .short('r')
                        .long("reason")
                        .help("Optional Reason for requesting relationship"),
                    Arg::new("generate-did")
                        .short('g')
                        .long("generate-did")
                        .help("Generate a new local relationship DID for this relationship request")
                        .default_value("true")
                        .action(ArgAction::SetFalse),
                ])
                .about("Request a new relationship")
                .arg_required_else_help(true),
        )
        .arg_required_else_help(true);

    // Full CLI Set
    Command::new("lkmv")
        .about("Linux Kernel Maintainer Verification")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand_required(true)
        .args([
            Arg::new("unlock-code")
                .short('u')
                .long("unlock-code")
                .help("If using unlock codes, can specify it here"),
            Arg::new("profile")
                .short('p')
                .long("profile")
                .help("Config profile to use")
                .default_value("default"),
        ])
        .subcommand(Command::new("status").about("Displays status of the lkmv tool"))
        .subcommand(
            Command::new("setup")
                .about("Initial configuration of the lkmv tool")
                .subcommand(
                    Command::new("import").about("Import settings").args([
                        Arg::new("file")
                            .short('f')
                            .long("file")
                            .default_value("export.lkmv")
                            .help("File containing exported settings"),
                        Arg::new("passphrase")
                            .short('p')
                            .long("passphrase")
                            .help("Passphrase to unlock the exported settings with"),
                    ]),
                ),
        )
        .subcommands([
            export_subcommand,
            contacts_subcommand,
            relationships_subcommand,
        ])
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

/// Loads lkmv with Trust Development Kit (TDK) and Config
/// This does not need to be called for setup!
async fn load(term: &Term, profile: &str) -> Result<(TDK, Config)> {
    // Instantiate the TDK
    let mut tdk = TDK::new(
        TDKConfigBuilder::new()
            .with_load_environment(false)
            .build()?,
        None,
    )
    .await?;

    let config = match Config::load(
        term,
        &mut tdk,
        profile,
        cli()
            .get_matches()
            .get_one::<String>("unlock-code")
            .map(|s| s.as_str()),
    )
    .await
    {
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

    Ok((tdk, config))
}

/// Checks if another instance of lkmv is running for the same profile
/// will return an error if a duplicate instance is found
/// otherwise, creates a lock file to prvent other instances from running
/// Returns the path to the lock file created
fn check_duplicate_instance(profile: &str) -> Result<String> {
    let lock_file = get_lock_file(profile)?;

    // Check if existing lockfile exists
    // If so, then check if the PID is still running
    match fs::exists(&lock_file) {
        Ok(exists) => {
            if exists {
                // Check the PID
                let pid = fs::read_to_string(&lock_file)?.trim_end().to_string();

                // We want to only refresh processes.
                let system = System::new_with_specifics(
                    RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
                );
                if system.process(Pid::from_str(&pid)?).is_some() {
                    println!(
                        "{}{}{} {}",
                        style("ERROR: Another instance of lkmv is running for this profile (")
                            .color256(CLI_RED),
                        style(profile).color256(CLI_PURPLE),
                        style(")!").color256(CLI_RED),
                        style(
                            "Only a single instance of lkmv can run for a given profile at a time!"
                        )
                        .color256(CLI_ORANGE)
                    );
                    bail!("Duplicate lkmv instance running");
                }
            }
        }
        Err(e) => {
            println!(
                "{} {}",
                style("ERROR: Couldn't check for lock file! Reason:").color256(CLI_RED),
                style(e).color256(CLI_ORANGE)
            );
            bail!("Lock File Error");
        }
    }

    // Create the lock file
    create_lock_file(&lock_file)?;
    Ok(lock_file)
}

/// Returns the path to the lock file for the given profile
fn get_lock_file(profile: &str) -> Result<String> {
    let path = if let Ok(config_path) = env::var("LKMV_CONFIG_PATH") {
        if config_path.ends_with('/') {
            config_path
        } else {
            [&config_path, "/"].concat()
        }
    } else if let Some(home) = dirs::home_dir()
        && let Some(home_str) = home.to_str()
    {
        [home_str, "/.config/lkmv/"].concat()
    } else {
        bail!("Couldn't determine Home directory");
    };

    if profile == "default" {
        Ok([&path, "config.lock"].concat())
    } else {
        Ok([&path, "config-", profile, ".lock"].concat())
    }
}

/// Creates the lock file containg the running process PID
fn create_lock_file(lock_file: &str) -> Result<()> {
    Ok(fs::write(lock_file, process::id().to_string())?)
}

/// Removes the lock file for the given profile
fn remove_lock_file(lock_file: &str) {
    let _ = fs::remove_file(lock_file);
}

#[tokio::main]
async fn main() -> Result<()> {
    let term = Term::stdout();

    // Which configuration profile to use?
    let profile = if let Ok(profile) = env::var("LKMV_CONFIG_PROFILE") {
        // ENV Profile will override the CLI Argument
        if cli()
            .get_matches()
            .get_one::<String>("profile")
            .unwrap_or(&"default".to_string())
            .as_str()
            != profile
        {
            println!("{}", 
                style("WARNING: Using both ENV LKMV_CONFIG_PROFILE and CLI profile! These do not match!").color256(CLI_ORANGE)
            );
            println!(
                "{} {}",
                style("WARNING: Using ENV Profile:").color256(CLI_ORANGE),
                style(&profile).color256(CLI_PURPLE)
            );
        }
        profile
    } else {
        cli()
            .get_matches()
            .get_one::<String>("profile")
            .unwrap_or(&"default".to_string())
            .to_string()
    };

    // Check if profile is currently active elsewhere?
    let lock_file = check_duplicate_instance(&profile)?;

    initialize(&term);

    // lkmv routines
    let result = lkmv(&term, &profile).await;

    remove_lock_file(&lock_file);

    result
}

async fn lkmv(term: &Term, profile: &str) -> Result<()> {
    match cli().get_matches().subcommand() {
        Some(("status", _)) => {
            let mut tdk = TDK::new(
                TDKConfigBuilder::new()
                    .with_load_environment(false)
                    .build()?,
                None,
            )
            .await?;
            print_status(
                term,
                &mut tdk,
                cli()
                    .get_matches()
                    .get_one::<String>("unlock-code")
                    .map(|s| s.as_str()),
                profile,
            )
            .await;
        }
        Some(("setup", args)) => {
            if let Some(args) = args.subcommand_matches("import") {
                let passphrase = args.get_one::<String>("passphrase");
                return Config::import(
                    passphrase.map(|s| SecretString::new(s.to_string())),
                    args.get_one::<String>("file")
                        .expect("No file specified!")
                        .as_ref(),
                    profile,
                );
            }
            match cli_setup(term, profile).await {
                Ok(_) => {
                    println!(
                        "\n{}",
                        style("Setup completed successfully.").color256(CLI_GREEN)
                    );
                }
                Err(e) => {
                    eprintln!("Setup failed: {e}");
                }
            }
        }
        Some(("export", args)) => {
            let (tdk, config) = load(term, profile).await?;

            match args.subcommand() {
                Some(("pgp-keys", sub_args)) => {
                    // Export PGP Keys
                    let user_id = sub_args.get_one::<String>("user-id");
                    let passphrase = sub_args.get_one::<String>("passphrase");

                    ask_export_community_did_keys(
                        term,
                        &config.get_community_keys(&tdk).await?,
                        user_id.map(|s| s.as_str()),
                        passphrase.map(|s| SecretString::new(s.to_string())),
                        false, // Not running in wizard mode
                    );
                }
                Some(("settings", sub_args)) => {
                    // Export settings
                    let passphrase = sub_args.get_one::<String>("passphrase");
                    config.export(
                        passphrase.map(|s| SecretString::new(s.to_string())),
                        sub_args
                            .get_one::<String>("file")
                            .expect("Code error - file should has a default!")
                            .as_str(),
                    );
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
        Some(("contacts", args)) => {
            let (tdk, mut config) = load(term, profile).await?;

            if config.contacts.contacts_entry(tdk, args).await? {
                // Need to save config
                config.save(profile)?;
            }
        }
        Some(("relationships", args)) => {
            let (tdk, mut config) = load(term, profile).await?;

            relationships_entry(tdk, &mut config, profile, args).await?;
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
