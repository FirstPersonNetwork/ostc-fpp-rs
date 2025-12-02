/*! Visual representation of Verifiable Credentials (VRC)
*/

use std::{collections::HashSet, rc::Rc};

use chrono::Local;
use console::style;

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_RED, CLI_WHITE, config::Config,
    relationships::Relationship, vrc::Vrc,
};

/// Shows all VRC's on screen
pub fn vrcs_show_all(config: &Config) {
    // Merge the keys from both issued and received VRC's together
    let mut keys: HashSet<Rc<String>> = config.private.vrcs_received.keys().cloned().collect();

    keys.extend(
        config
            .private
            .vrcs_issued
            .keys()
            .cloned()
            .collect::<HashSet<Rc<String>>>(),
    );

    for remote in keys {
        vrcs_show_relationship(&remote, config);
    }
}

/// Shows all VRC's for a relationship
/// remote: Must be the remote DID of the relationship (can be R-DID or C-DID)
pub fn vrcs_show_relationship(remote: &Rc<String>, config: &Config) {
    let relationship: Relationship =
        if let Some(relationship) = config.private.relationships.find_by_remote_did(remote) {
            let guard = relationship.lock().unwrap();
            guard.clone()
        } else {
            println!(
                "{}{}",
                style("ERROR: Missing relationship record for DID: ").color256(CLI_RED),
                style(remote.as_str()).color256(CLI_ORANGE)
            );
            return;
        };

    let Some(contact) = config
        .private
        .contacts
        .find_contact(&relationship.remote_c_did)
    else {
        println!(
            "{}{}",
            style("ERROR: Missing contact record for DID: ").color256(CLI_RED),
            style(&relationship.remote_c_did).color256(CLI_ORANGE)
        );
        return;
    };

    println!();
    print!(
        "{}{} {}{}",
        style("Relationship Alias: ").color256(CLI_BLUE).bold(),
        if let Some(alias) = &contact.alias {
            style(alias.as_str()).color256(CLI_GREEN)
        } else {
            style("<No Alias>").color256(CLI_ORANGE).italic()
        },
        style("Community DID: ").color256(CLI_BLUE).bold(),
        style(&relationship.remote_c_did).color256(CLI_PURPLE)
    );
    println!();

    println!(
        "{}{}",
        style("<-- ").color256(CLI_BLUE).bold(),
        style("You have issued the following VRC's to this Relationship:")
            .color256(CLI_BLUE)
            .bold()
            .underlined()
    );
    if let Some(vrcs) = config.private.vrcs_issued.get(remote)
        && !vrcs.is_empty()
    {
        for (vrc_id, vrc) in vrcs {
            vrc_show(vrc_id, vrc);
            println!();
        }
    } else {
        println!(
            "\t{}",
            style("You haven't issued any VRC's for this relationship").color256(CLI_ORANGE)
        );
        println!();
    }

    println!(
        "{}{}",
        style("--> ").color256(CLI_BLUE).bold(),
        style("You have received the following VRC's for this Relationship:")
            .color256(CLI_BLUE)
            .bold()
            .underlined()
    );
    if let Some(vrcs) = config.private.vrcs_received.get(remote)
        && !vrcs.is_empty()
    {
        for (vrc_id, vrc) in vrcs {
            vrc_show(vrc_id, vrc);
            println!();
        }
    } else {
        println!(
            "\t{}",
            style("You haven't received any VRC's for this relationship").color256(CLI_ORANGE)
        );
        println!();
    }
}

/// Prints a vrc to the screen
pub fn vrc_show(vrc_id: &str, vrc: &Vrc) {
    println!(
        "\t{}{}",
        style("VRC ID: ").color256(CLI_BLUE).bold(),
        style(vrc_id).color256(CLI_PURPLE)
    );

    print!("\t  {}", style("Name: ").color256(CLI_BLUE).bold());
    if let Some(name) = &vrc.name {
        print!("{}", style(name).color256(CLI_WHITE));
    } else {
        print!("{}", style("N/A").color256(CLI_ORANGE));
    }
    println!();

    print!("\t  {}", style("Description: ").color256(CLI_BLUE).bold());
    if let Some(description) = &vrc.description {
        print!("{}", style(description).color256(CLI_WHITE));
    } else {
        print!("{}", style("N/A").color256(CLI_ORANGE));
    }
    println!();

    if let Some(rel_type) = &vrc.credential_subject.relationship_type {
        println!(
            "\t  {}{}",
            style("Relationship Type: ").color256(CLI_BLUE).bold(),
            style(rel_type).color256(CLI_WHITE)
        );
    }

    println!(
        "\t  {}{} {}{} {}{}",
        style("Valid From: ").color256(CLI_BLUE).bold(),
        style(
            &vrc.valid_from
                .with_timezone(&Local)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        )
        .color256(CLI_WHITE),
        style("Started?: ").color256(CLI_BLUE).bold(),
        if let Some(start_date) = vrc.credential_subject.start_date {
            style(
                start_date
                    .with_timezone(&Local)
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            )
            .color256(CLI_WHITE)
        } else {
            style("N/A".to_string()).color256(CLI_ORANGE)
        },
        style("End Date?: ").color256(CLI_BLUE).bold(),
        if let Some(end_date) = vrc.credential_subject.end_date {
            style(
                end_date
                    .with_timezone(&Local)
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            )
            .color256(CLI_WHITE)
        } else {
            style("N/A".to_string()).color256(CLI_ORANGE)
        },
    );
}

/// Prints a VRC JSON to screen
pub fn show_vrc_by_id(config: &Config, id: &str) {
    if let Some(vrc) = config.vrcs.get(&Rc::new(id.to_string())) {
        println!(
            "{}{}\n{}",
            style("VRC ID: ").color256(CLI_BLUE).bold(),
            style(id).color256(CLI_PURPLE),
            style(serde_json::to_string_pretty(&vrc).unwrap()).color256(CLI_WHITE)
        )
    } else {
        println!(
            "{}{}",
            style("ERROR: No VRC found with ID: ").color256(CLI_RED),
            style(id).color256(CLI_ORANGE)
        )
    }
}
