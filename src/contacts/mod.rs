/*!
*  Managing known contacts is useful and easy to establish relationships with others
*/

use crate::{CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_RED};
use affinidi_tdk::TDK;
use anyhow::{Result, bail};
use clap::{ArgMatches, Id};
use console::style;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, rc::Rc};

/// A record for a single known Contact
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Contact {
    /// DID representing the contact
    pub did: Rc<String>,

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
    pub contacts: HashMap<Rc<String>, Rc<Contact>>,

    /// Helps with finding a DID by it's alias
    #[serde(skip)]
    pub aliases: HashMap<String, Rc<Contact>>,
}

impl Contacts {
    /// Primary entry point for all Contact Management related functionality
    /// Returns true if config changed and needs to be saved
    pub async fn contacts_entry(&mut self, tdk: TDK, args: &ArgMatches) -> Result<bool> {
        Ok(match args.subcommand() {
            Some(("add", sub_args)) => {
                let did = if let Some(did) = sub_args.get_one::<String>("did") {
                    did.to_string()
                } else {
                    println!(
                        "{}",
                        style("ERROR: You must specify a DID to add!").color256(CLI_RED)
                    );
                    bail!("Contact DID is required");
                };
                let alias = sub_args.get_one::<String>("alias");
                let skip = sub_args.get_flag("skip");

                self.add_contact(&tdk, &did, alias.map(|s| s.to_string()), skip)
                    .await?;

                println!(
                    "{}",
                    style("Successfully added new contact").color256(CLI_GREEN)
                );
                if let Some(alias) = alias {
                    print!(
                        "  {}{}{}",
                        style("alias (").color256(CLI_BLUE),
                        style(alias).color256(CLI_PURPLE),
                        style(")").color256(CLI_BLUE),
                    );
                } else {
                    print!(
                        "  {}{}{}",
                        style("alias (").color256(CLI_BLUE),
                        style("NONE").color256(CLI_ORANGE),
                        style(")").color256(CLI_BLUE),
                    );
                }
                println!(
                    " {}{}{}",
                    style("contact DID (").color256(CLI_BLUE),
                    style(did).color256(CLI_PURPLE),
                    style(")").color256(CLI_BLUE),
                );
                true
            }
            Some(("remove", sub_args)) => {
                let did = sub_args.get_one::<String>("did").map(|s| s.to_string());
                let alias = sub_args.get_one::<String>("alias").map(|s| s.to_string());
                let name = sub_args
                    .get_one::<Id>("remove-by")
                    .expect("No valid contact name to remove")
                    .as_str();

                let changed = self.remove_contact(did, alias);

                if let Some(changed) = changed {
                    println!(
                        "{}{}{}",
                        style("Successfully removed contact (").color256(CLI_GREEN),
                        style(&changed.did).color256(CLI_PURPLE),
                        style(")").color256(CLI_GREEN)
                    );
                    true
                } else {
                    println!(
                        "{}{}{}",
                        style("No contact found that matched (").color256(CLI_ORANGE),
                        style(name).color256(CLI_PURPLE),
                        style(")").color256(CLI_ORANGE)
                    );
                    false
                }
            }
            Some(("list", _)) => {
                self.print_list();
                false
            }
            _ => {
                println!(
                    "{} {}",
                    style("ERROR:").color256(CLI_RED),
                    style(
                        "No valid contacts subcommand was used. Use --help for more information."
                    )
                    .color256(CLI_ORANGE)
                );
                false
            }
        })
    }

    pub fn is_empty(&self) -> bool {
        self.contacts.is_empty()
    }

    /// Adds a new contact
    /// tdk: Trust Development Kit instance
    /// contact_did: DID of the contact to add
    /// alias: Optional alias for the contact
    /// check_did: Whether to check if the DID is valid
    pub async fn add_contact(
        &mut self,
        tdk: &TDK,
        contact_did: &str,
        alias: Option<String>,
        check_did: bool,
    ) -> Result<Rc<Contact>> {
        if check_did {
            match tdk.did_resolver().resolve(contact_did).await {
                Ok(_) => {}
                Err(e) => {
                    println!(
                        "{}{}{}",
                        style("ERROR: Couldn't resolve DID ").color256(CLI_RED),
                        style(contact_did).color256(CLI_PURPLE),
                        style(format!(" Reason: {}", e)).color256(CLI_ORANGE)
                    );
                    bail!("Could not resolve DID");
                }
            }
        }

        let contact_did = Rc::new(contact_did.to_string());

        if let Some(alias) = &alias
            && self.aliases.contains_key(alias)
        {
            println!(
                "{} {}{}{}",
                style("ERROR: Duplicate alias detected!").color256(CLI_RED),
                style("alias(").color256(CLI_ORANGE),
                style(alias).color256(CLI_PURPLE),
                style(") must be removed first").color256(CLI_ORANGE)
            );
            bail!("Duplicate alias detected")
        }

        let contact = Rc::new(Contact {
            did: contact_did.clone(),
            alias: alias.clone(),
        });

        self.contacts.insert(contact_did, contact.clone());

        if let Some(alias) = alias {
            self.aliases.insert(alias, contact.clone());
        }

        Ok(contact)
    }

    /// Removes a contact (by DID or Alias)
    /// Returns Contact if contact was found and removed
    fn remove_contact(
        &mut self,
        contact_did: Option<String>,
        alias: Option<String>,
    ) -> Option<Rc<Contact>> {
        if let Some(contact_did) = contact_did {
            if let Some(contact) = self.contacts.get(&contact_did)
                && let Some(alias) = &contact.alias
            {
                self.aliases.remove(alias);
            }

            self.contacts.remove(&contact_did)
        } else if let Some(alias) = alias {
            self.aliases.remove(&alias).inspect(|contact| {
                self.contacts.remove(&contact.did);
            })
        } else {
            println!("{}", style("ERROR: Somehow no did or alias was specified when deleting a contact! This is a code error!").color256(CLI_RED));
            None
        }
    }

    // Dumps contct information to the console
    fn print_list(&self) {
        if self.is_empty() {
            println!(
                "{}",
                style("There are no known contacts").color256(CLI_ORANGE)
            );
            return;
        }

        for contact in self.contacts.values() {
            if let Some(alias) = &contact.alias {
                print!(
                    "  {}{}{}",
                    style("alias (").color256(CLI_BLUE),
                    style(alias).color256(CLI_PURPLE),
                    style(")").color256(CLI_BLUE),
                );
            } else {
                print!(
                    "  {}{}{}",
                    style("alias (").color256(CLI_BLUE),
                    style("NONE").color256(CLI_ORANGE),
                    style(")").color256(CLI_BLUE),
                );
            }
            println!(
                " {}{}{}",
                style("contact DID (").color256(CLI_BLUE),
                style(&contact.did).color256(CLI_PURPLE),
                style(")").color256(CLI_BLUE),
            );
        }
    }

    /// Finds a contact by alias or DID
    /// will look for alias first, then DID
    pub fn find_contact(&self, id: &str) -> Option<Rc<Contact>> {
        if let Some(contact) = self.aliases.get(id) {
            Some(contact.clone())
        } else {
            #[allow(clippy::unnecessary_to_owned)] // Because using RC's
            self.contacts.get(&(id.to_string())).cloned()
        }
    }
}

/// Private Shadow struct to help with deserializing Contacts and recreating the aliases map
#[derive(Deserialize)]
struct ContactsShadow {
    contacts: HashMap<Rc<String>, Rc<Contact>>,
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
