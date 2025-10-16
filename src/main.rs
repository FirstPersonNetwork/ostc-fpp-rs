/* Linux Kernel Maintainer Verification Tool
*
*/

use crate::setup::cli_setup;
use clap::Command;
use console::{Term, style};
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

fn cli() -> Command {
    Command::new("lkmv")
        .about("Linux Kernel Maintainer Verification")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(Command::new("status").about("Displays status of the lkmv tool"))
        .subcommand(Command::new("setup").about("Initial setup of the lkmv configuration"))
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

fn main() {
    let term = Term::stdout();

    initialize(&term);

    match cli().get_matches().subcommand() {
        Some(("status", _)) => {
            print_status();
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
        _ => {
            eprintln!("No valid subcommand was used. Use --help for more information.");
        }
    }
}
