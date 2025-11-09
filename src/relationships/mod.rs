/*!
*   Relationship Management
*/

use crate::{CLI_ORANGE, CLI_RED, config::Config, relationships::request::create_request};
use affinidi_tdk::TDK;
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use console::style;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

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
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(from = "RelationshipsShadow")]
pub struct Relationships {
    /// Mapping relationships by the remote R-DID
    pub relationships: HashMap<Rc<String>, Rc<Relationship>>,

    /// Mapping relationships by our R-DIDs
    pub r_map: HashMap<Rc<String>, Vec<HashSet<Rc<Relationship>>>>,

    /// latest BIP32 path pointer to use for new keys
    pub path_pointer: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Relationship {
    /// What DID are we using in this relationship?
    pub our_did: Rc<String>,

    /// What is the DID of the remote party in this relationship?
    pub remote_did: Rc<String>,

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
        let mut r_map: HashMap<Rc<String>, Vec<HashSet<Rc<Relationship>>>> = HashMap::new();

        for relationship in value.relationships {
            relationships.insert(relationship.remote_did.clone(), relationship.clone());

            r_map
                .entry(relationship.our_did.clone())
                .or_default()
                .push(HashSet::from([relationship.clone()]));
        }

        Relationships {
            relationships,
            r_map,
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
pub fn create_relationship_did(config: &mut Config, mediator: &str) -> Result<String> {
    // Derive a key path

    todo!("Need to finish");
}
