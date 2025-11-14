/*! Command Line Interface configuration
*/

use clap::{Arg, ArgAction, Command};

pub fn cli() -> Command {
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
        .subcommand(Command::new("list").about("Lists all known contacts"))
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
        .arg_required_else_help(true);

    // Relationship management
    let relationships_subcommand = Command::new("relationships")
        .about("Manage relationships")
        .subcommand(Command::new("list").about("List Relationships"))
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

    // Tasks management
    let tasks_subcommand = Command::new("tasks")
        .about("Manage tasks")
        .subcommand(Command::new("fetch").about("Fetches tasks that are awaiting attention"))
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
            tasks_subcommand,
        ])
}
