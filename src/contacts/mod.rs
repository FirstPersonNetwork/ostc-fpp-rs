/*!
*  Managing known contacts is useful and easy to establish relationships with others
*/

use crate::{CLI_ORANGE, CLI_RED, config::Config};
use affinidi_tdk::TDK;
use anyhow::Result;
use clap::ArgMatches;
use console::{Term, style};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, rc::Rc};

/// Primary entry point for all Contact Management related functionality
pub async fn contacts_entry(
    term: &Term,
    tdk: TDK,
    config: &mut Config,
    args: &ArgMatches,
) -> Result<()> {
    match args.subcommand() {
        Some(("add", sub_args)) => {
            let did = sub_args.get_one::<String>("did");
            let alias = sub_args.get_one::<String>("alias");

            todo!("contacts add");
        }
        Some(("remove", sub_args)) => {
            todo!("contacts remove");
        }
        _ => {
            println!(
                "{} {}",
                style("ERROR:").color256(CLI_RED),
                style("No valid contacts subcommand was used. Use --help for more information.")
                    .color256(CLI_ORANGE)
            );
        }
    }

    Ok(())
}

/// A record for a single known Contact
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Contact {
    /// DID representing the contact
    pub did: String,

    /// Optional alias for the DID
    pub alias: Option<String>,
}

// ****************************************************************************
// Contacts Collection
// ****************************************************************************

/// Contains all known contacts
/// Uses Reference Counters to avoid duplicating Contact instances
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(from = "ContactsShadow")]
pub struct Contacts {
    /// Contacts with key being DID
    pub contacts: HashMap<String, Rc<Contact>>,

    /// Helps with finding a DID by it's alias
    #[serde(skip)]
    pub aliases: HashMap<String, Rc<Contact>>,
}

/// Private Shadow struct to help with deserializing Contacts and recreating the aliases map
#[derive(Deserialize)]
struct ContactsShadow {
    contacts: HashMap<String, Rc<Contact>>,
}

impl From<ContactsShadow> for Contacts {
    fn from(shadow: ContactsShadow) -> Self {
        let mut contacts = Contacts {
            contacts: shadow.contacts,
            aliases: HashMap::new(),
        };

        for contact in contacts.contacts.values() {
            if let Some(alias) = &contact.alias {
                contacts.aliases.insert(alias.clone(), contact.clone());
            }
        }

        contacts
    }
}
