/* Linux Kernel Maintainer Verification Tool
*
*/

use clap::Command;
use status::print_status;
use tracing_subscriber::EnvFilter;

mod status;

// CLI Color codes
const CLI_GREEN: u8 = 34;
const CLI_BLUE: u8 = 69;
const CLI_RED: u8 = 9;

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
fn setup() {
    // Setup logging/tracing
    // If no RUST_LOG ENV variable is set, defaults to MAX_LEVEL: ERROR
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

fn main() {
    setup();

    match cli().get_matches().subcommand() {
        Some(("status", _)) => {
            print_status();
        }
        _ => {
            eprintln!("No valid subcommand was used. Use --help for more information.");
        }
    }
}
