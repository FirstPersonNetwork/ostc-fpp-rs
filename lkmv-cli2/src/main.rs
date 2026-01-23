use crate::{cli::cli, state_handler::StateHandler, ui::UiManager};
use anyhow::{Context, Result, bail};
use console::style;
use lkmv::colors::{CLI_BLUE, CLI_ORANGE, CLI_PURPLE, CLI_RED};
use std::{env, fs, path::Path, process, str::FromStr};
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
#[cfg(unix)]
use tokio::signal::unix::signal;
use tokio::sync::broadcast;

mod cli;
mod state_handler;
mod ui;

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
// MAIN Function
// ****************************************************************************

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    // Setup the initial state
    let (terminator, mut interrupt_rx) = create_termination();
    let (state, state_rx) = StateHandler::new(&profile);
    let (ui_manager, action_rx) = UiManager::new();

    tokio::try_join!(
        state.main_loop(terminator, action_rx, interrupt_rx.resubscribe()),
        ui_manager.main_loop(state_rx, interrupt_rx.resubscribe()),
    )?;

    match interrupt_rx.recv().await {
        Ok(reason) => match reason {
            Interrupted::UserInt => println!("exited per user request"),
            Interrupted::OsSigInt => println!("exited because of an os sig int"),
            Interrupted::SystemError(reason) => {
                println!("exited because of a system error: {reason}")
            }
        },
        _ => {
            println!("exited because of an unexpected error");
        }
    }

    remove_lock_file(&lock_file);
    Ok(())
}

// ****************************************************************************
// Termination Management
// ****************************************************************************

#[derive(Debug, Clone)]
pub enum Interrupted {
    OsSigInt,
    UserInt,
    SystemError(String),
}

#[derive(Debug, Clone)]
pub struct Terminator {
    interrupt_tx: broadcast::Sender<Interrupted>,
}

impl Terminator {
    pub fn new(interrupt_tx: broadcast::Sender<Interrupted>) -> Self {
        Self { interrupt_tx }
    }

    pub fn terminate(&mut self, interrupted: Interrupted) -> anyhow::Result<()> {
        self.interrupt_tx.send(interrupted)?;

        Ok(())
    }
}

#[cfg(unix)]
async fn terminate_by_unix_signal(mut terminator: Terminator) {
    let mut interrupt_signal = signal(tokio::signal::unix::SignalKind::interrupt())
        .expect("failed to create interrupt signal stream");

    interrupt_signal.recv().await;

    terminator
        .terminate(Interrupted::OsSigInt)
        .expect("failed to send interrupt signal");
}

// create a broadcast channel for retrieving the application kill signal
pub fn create_termination() -> (Terminator, broadcast::Receiver<Interrupted>) {
    let (tx, rx) = broadcast::channel(1);
    let terminator = Terminator::new(tx);

    #[cfg(unix)]
    tokio::spawn(terminate_by_unix_signal(terminator.clone()));

    (terminator, rx)
}
