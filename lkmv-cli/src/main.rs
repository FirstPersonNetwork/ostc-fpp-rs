/* Linux Kernel Maintainer Verification Tool
*
*/

use crate::{
    cli::cli,
    config::Config,
    interactions::vrc::vrcs_entry,
    maintainers::maintainers_entry,
    relationships::relationships_entry,
    setup::{cli_setup, pgp_export::ask_export_persona_did_keys},
    tasks::tasks_entry,
};
use affinidi_tdk::{TDK, common::config::TDKConfigBuilder};
use anyhow::{Context, Result, bail};
use console::{Term, style};
use dialoguer::{Password, theme::ColorfulTheme};
use secrecy::SecretString;
use sha2::Digest;
use status::print_status;
use std::{env, fs, path::Path, process, str::FromStr};
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
use tracing_subscriber::EnvFilter;

mod cli;
mod config;
mod contacts;
mod interactions;
mod log;
mod maintainers;
mod messaging;
#[cfg(feature = "openpgp-card")]
mod openpgp_card;
mod relationships;
mod setup;
mod status;
mod tasks;

// CLI Color codes
const CLI_BLUE: u8 = 69; // Use for general information
const CLI_GREEN: u8 = 34; // Use for Successful text
const CLI_RED: u8 = 9; // Use for Error messages
const CLI_ORANGE: u8 = 214; // Use for cautionary data
const CLI_PURPLE: u8 = 165; // Use for Example data
const CLI_WHITE: u8 = 15;

// Primary Linux Kernel Mediator DID
const LF_PUBLIC_MEDIATOR_DID: &str =
    "did:webvh:QmetnhxzJXTJ9pyXR1BbZ2h6DomY6SB1ZbzFPrjYyaEq9V:fpp.storm.ws:public-mediator";

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
                let pid = fs::read_to_string(&lock_file)
                    .context("Couldn't read from lockfile")?
                    .trim_end()
                    .to_string();

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
    create_lock_file(&lock_file).context("create_lock_file() failed")?;
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
    let dir_path = Path::new(&lock_file);

    // Check that directory structure exists
    if let Some(parent_path) = dir_path.parent()
        && !parent_path.exists()
    {
        // Create parent directories
        fs::create_dir_all(parent_path)?;
    }

    match fs::write(lock_file, process::id().to_string()) {
        Ok(_) => Ok(()),
        Err(e) => {
            println!(
                "{}{}{}{}",
                style("ERROR: Couldn't create lock file: ").color256(CLI_RED),
                style(lock_file).color256(CLI_PURPLE),
                style(" Reason: ").color256(CLI_RED),
                style(e).color256(CLI_ORANGE)
            );
            bail!("Couldn't create lock file");
        }
    }
}

/// Removes the lock file for the given profile
fn remove_lock_file(lock_file: &str) {
    let _ = fs::remove_file(lock_file);
}

// ****************************************************************************
// MAIN FUNCTION
// ****************************************************************************
#[tokio::main]
async fn main() -> Result<()> {
    let term = Term::stdout();

    // Which configuration profile to use?
    let profile = if let Ok(env_profile) = env::var("LKMV_CONFIG_PROFILE") {
        // ENV Profile will override the CLI Argument
        let cli_profile = cli()
            .get_matches()
            .get_one::<String>("profile")
            .unwrap_or(&"default".to_string())
            .to_string();
        if cli_profile != "default" && cli_profile != env_profile {
            println!("{}", 
                style("WARNING: Using both ENV LKMV_CONFIG_PROFILE and CLI profile! These do not match!").color256(CLI_ORANGE)
            );
            println!(
                "{} {}",
                style("WARNING: Using CLI Profile:").color256(CLI_ORANGE),
                style(&cli_profile).color256(CLI_PURPLE)
            );
            cli_profile
        } else {
            println!(
                "{}{}{}",
                style("Using profile (").color256(CLI_BLUE),
                style(&env_profile).color256(CLI_PURPLE),
                style(") from LKMV_CONFIG_PROFILE ENV variable").color256(CLI_BLUE)
            );
            env_profile
        }
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
        Some(("logs", _)) => {
            let (_, config) = load(term, profile).await?;

            config.public.logs.show_all();
        }
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

                    ask_export_persona_did_keys(
                        term,
                        &config.get_persona_keys(&tdk).await?,
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

            if config
                .private
                .contacts
                .contacts_entry(
                    tdk,
                    args,
                    &config.private.relationships,
                    &mut config.public.logs,
                )
                .await?
            {
                // Need to save config
                config.save(profile)?;
            }
        }
        Some(("relationships", args)) => {
            let (tdk, mut config) = load(term, profile).await?;

            relationships_entry(tdk, &mut config, profile, args).await?;
        }
        Some(("tasks", args)) => {
            let (tdk, mut config) = load(term, profile).await?;

            tasks_entry(tdk, &mut config, profile, args, term).await?;
        }
        Some(("vrcs", args)) => {
            let (tdk, mut config) = load(term, profile).await?;

            vrcs_entry(tdk, &mut config, profile, args).await?;
        }
        Some(("maintainers", args)) => {
            let (tdk, mut config) = load(term, profile).await?;

            maintainers_entry(tdk, &mut config, profile, args).await?;
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
