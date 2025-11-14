/*!
*   Relationship Management
*/

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_RED,
    config::{
        Config, KeyTypes,
        secured_config::{KeyInfoConfig, KeySourceMaterial},
    },
    contacts::Contacts,
    relationships::request::create_request,
};
use affinidi_tdk::{
    TDK,
    did_peer::DIDPeerKeys,
    dids::DID,
    secrets_resolver::{SecretsResolver, secrets::Secret},
};
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use console::style;
use ed25519_dalek_bip32::DerivationPath;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, rc::Rc};

mod request;

#[derive(Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationshipState {
    /// Relationship Request has been sent to the remote party
    RequestSent,

    /// Relationship Request has been accepted by respondent, need to finalise the relationship
    /// still
    RequestAccepted,

    /// Relationship Rejected by respondent
    RequestRejected,

    /// Relationship is established
    Established,

    /// There is no relationship
    None,
}

impl Display for RelationshipState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state_str = match self {
            RelationshipState::RequestSent => "Request Sent",
            RelationshipState::RequestAccepted => "Request Accepted",
            RelationshipState::RequestRejected => "Request Rejected",
            RelationshipState::Established => "Established",
            RelationshipState::None => "None",
        };
        write!(f, "{}", state_str)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(from = "RelationshipsShadow", into = "RelationshipsShadow")]
pub struct Relationships {
    /// Mapping relationships by the remote R-DID
    pub relationships: HashMap<Rc<String>, Rc<Relationship>>,

    /*
    /// Mapping relationships by our R-DIDs
    pub r_map: HashMap<Rc<String>, Vec<HashSet<Rc<Relationship>>>>,
    */
    /// latest BIP32 path pointer to use for new keys
    pub path_pointer: u32,
}

impl Relationships {
    /// Prints Relationship status to the console
    pub fn status(&self, contacts: &Contacts) {
        println!("{}", style("Relationships").bold().color256(CLI_BLUE));
        println!("{}", style("=============").bold().color256(CLI_BLUE));

        println!(
            "{} {}",
            style("Relationships path pointer: ").color256(CLI_BLUE),
            style(self.path_pointer).color256(CLI_GREEN)
        );

        if self.relationships.is_empty() {
            println!(
                "{}",
                style("No relationships established yet.").color256(CLI_ORANGE)
            );
            return;
        }

        println!("{}", style("Relationships").color256(CLI_BLUE));
        self.print_relationships(contacts);
    }

    pub fn print_relationships(&self, contacts: &Contacts) {
        if self.relationships.is_empty() {
            println!("{}", style("No relationships exist").color256(CLI_ORANGE));
        } else {
            for r in self.relationships.values() {
                let remote_c_did_alias = if let Some(contact) =
                    contacts.find_contact(&r.remote_c_did)
                    && let Some(alias) = &contact.alias
                {
                    style(alias.to_string()).color256(CLI_GREEN)
                } else {
                    style("N/A".to_string()).color256(CLI_ORANGE)
                };

                println!(
                    "  {}{}{}{}",
                    style("Remote DID: Alias: ").color256(CLI_BLUE),
                    remote_c_did_alias,
                    style(" Community DID: ").color256(CLI_BLUE),
                    style(&r.remote_c_did).color256(CLI_GREEN),
                );

                if r.remote_did != r.remote_c_did {
                    println!(
                        "    {}{}",
                        style("Using r-did: ").color256(CLI_BLUE),
                        style(&r.remote_did).color256(CLI_PURPLE)
                    );
                }
                println!(
                    "    {}{}{}{}",
                    style("State: ").color256(CLI_BLUE),
                    style(&r.state).color256(CLI_GREEN),
                    style(" Created: ").color256(CLI_BLUE),
                    style(r.created).color256(CLI_GREEN)
                );
                println!();
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Relationship {
    /// What DID are we using in this relationship?
    pub our_did: Rc<String>,

    /// What is the DID of the remote party in this relationship?
    pub remote_did: Rc<String>,

    /// What is the remote end community DID?
    /// NOTE: This may be the same as the remote did itself, or it may be a random r-did
    pub remote_c_did: Rc<String>,

    /// When was this relationship created?
    pub created: DateTime<Utc>,

    /// State machine status of this relationship
    pub state: RelationshipState,
}

/// Used to serialize the more complex Relationships structure to SecuredConfig
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RelationshipsShadow {
    pub relationships: Vec<Rc<Relationship>>,
    pub path_pointer: u32,
}

impl From<Relationships> for RelationshipsShadow {
    fn from(value: Relationships) -> Self {
        let relationships = value
            .relationships
            .values()
            .cloned()
            .collect::<Vec<Rc<Relationship>>>();
        RelationshipsShadow {
            relationships,
            path_pointer: value.path_pointer,
        }
    }
}

impl From<RelationshipsShadow> for Relationships {
    fn from(value: RelationshipsShadow) -> Self {
        let mut relationships: HashMap<Rc<String>, Rc<Relationship>> = HashMap::new();
        //let mut r_map: HashMap<Rc<String>, Vec<HashSet<Rc<Relationship>>>> = HashMap::new();

        for relationship in value.relationships {
            relationships.insert(relationship.remote_did.clone(), relationship.clone());

            /*
                        r_map
                            .entry(relationship.our_did.clone())
                            .or_default()
                            .push(HashSet::from([relationship.clone()]));
            */
        }

        Relationships {
            relationships,
            //r_map,
            path_pointer: value.path_pointer,
        }
    }
}

// ****************************************************************************
// Primary entry point for Relationships from the CLI
// ****************************************************************************

/// Primary entry point for the Relationships module from the CLI
pub async fn relationships_entry(
    tdk: TDK,
    config: &mut Config,
    profile: &str,
    args: &ArgMatches,
) -> Result<()> {
    match args.subcommand() {
        Some(("list", _)) => {
            config
                .private
                .relationships
                .print_relationships(&config.private.contacts);
        }
        Some(("request", sub_args)) => {
            let respondent = if let Some(respondent) = sub_args.get_one::<String>("respondent") {
                respondent.to_string()
            } else {
                println!(
                        "{}",
                        style("ERROR: You must specify the respondent alias or DID! Otherwise you are going to be lonely..").color256(CLI_RED)
                    );
                bail!("Respondent alias or DID is required");
            };
            let alias = sub_args.get_one::<String>("alias");
            let reason = sub_args.get_one::<String>("reason");
            let generate_did = sub_args.get_flag("generate-did");

            create_request(
                tdk,
                config,
                &respondent,
                alias.map(|s| s.to_string()),
                reason.map(|s| s.as_str()),
                generate_did,
            )
            .await?;

            config.save(profile)?;
        }
        _ => {
            println!(
                "{} {}",
                style("ERROR:").color256(CLI_RED),
                style(
                    "No valid relationships subcommand was used. Use --help for more information."
                )
                .color256(CLI_ORANGE)
            );
        }
    }

    Ok(())
}

// ****************************************************************************
// Create relationship DID (random DID:PEER)
// ****************************************************************************

/// Creates a random did:peer DID representing a relationship DID
/// Add the keys used to the Configuration (you need to save config elsewhere after this)
pub async fn create_relationship_did(
    tdk: &TDK,
    config: &mut Config,
    mediator: &str,
) -> Result<String> {
    // Derive a key path
    let v_path = [
        "m/3'/1'/1'/",
        config
            .private
            .relationships
            .path_pointer
            .to_string()
            .as_str(),
        "'",
    ]
    .concat();
    config.private.relationships.path_pointer += 1;
    let e_path = [
        "m/3'/1'/1'/",
        config
            .private
            .relationships
            .path_pointer
            .to_string()
            .as_str(),
        "'",
    ]
    .concat();
    config.private.relationships.path_pointer += 1;

    let v_key = config
        .bip32_root
        .derive(&v_path.parse::<DerivationPath>()?)?;
    let e_key = config
        .bip32_root
        .derive(&e_path.parse::<DerivationPath>()?)?;

    let mut v_secret = Secret::generate_ed25519(None, Some(v_key.signing_key.as_bytes()));
    let mut e_secret = Secret::generate_x25519(None, Some(e_key.signing_key.as_bytes()))?;

    let mut keys = vec![
        (DIDPeerKeys::Verification, &mut v_secret),
        (DIDPeerKeys::Encryption, &mut e_secret),
    ];
    let r_did = match DID::generate_did_peer_from_secrets(&mut keys, Some(mediator.to_string())) {
        Ok(did) => did,
        Err(e) => {
            println!(
                "{} {}",
                style("ERROR: Failed to create relationship DID:").color256(CLI_RED),
                style(e.to_string()).color256(CLI_ORANGE)
            );
            bail!("Failed to create relationship DID");
        }
    };

    // Add the secrets to the config
    config.key_info.insert(
        v_secret.id.clone(),
        KeyInfoConfig {
            path: KeySourceMaterial::Derived { path: v_path },
            create_time: Utc::now(),
            purpose: KeyTypes::RelationshipVerification,
        },
    );
    config.key_info.insert(
        e_secret.id.clone(),
        KeyInfoConfig {
            path: KeySourceMaterial::Derived { path: e_path },
            create_time: Utc::now(),
            purpose: KeyTypes::RelationshipEncryption,
        },
    );

    // Add the secrets to the TDK secret resolver
    tdk.get_shared_state()
        .secrets_resolver
        .insert(v_secret)
        .await;
    tdk.get_shared_state()
        .secrets_resolver
        .insert(e_secret)
        .await;

    Ok(r_did)
}
