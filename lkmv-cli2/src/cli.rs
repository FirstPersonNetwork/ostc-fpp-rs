/*! Command Line Interface configuration
*/

use clap::{Arg, Command};

pub fn cli() -> Command {
    // Full CLI Set
    Command::new("lkmv")
        .about("Linux Kernel Maintainer Verification")
        .subcommand_required(false)
        .arg_required_else_help(false)
        .allow_external_subcommands(true)
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
}
