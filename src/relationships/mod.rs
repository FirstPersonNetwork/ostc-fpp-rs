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
    log::LogFamily,
    relationships::messages::create_send_request,
    setup::{KeyPurpose, bip32_bip39::Bip32Extension},
    tasks::TaskType,
};
use affinidi_tdk::{
    TDK,
    did_peer::DIDPeerKeys,
    didcomm::PackEncryptedOptions,
    dids::DID,
    messaging::{profiles::ATMProfile, protocols::Protocols},
    secrets_resolver::{
        SecretsResolver, crypto::ed25519::ed25519_private_to_x25519_private_key, secrets::Secret,
    },
};
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use console::style;
use ed25519_dalek_bip32::{DerivationPath, ExtendedSigningKey};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Display,
    rc::Rc,
    sync::{Arc, Mutex},
};

pub mod inbound;
pub mod messages;

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
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

// ****************************************************************************
// Message Body Structure types
// ****************************************************************************

/// DIDComm message body sent to the remote party when requesting a relationship
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RelationshipRequestBody {
    pub reason: Option<String>,
    pub did: String,
}

/// DIDComm message body sent to the initiator of a relationship request when the request is
/// rejected
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RelationshipRejectBody {
    pub reason: Option<String>,
}

/// Body of a Relationship Rquest accept message
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RelationshipAcceptBody {
    pub did: String,
}

// ****************************************************************************
// Relationships
// ****************************************************************************

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(from = "RelationshipsShadow", into = "RelationshipsShadow")]
pub struct Relationships {
    /// Mapping relationships by the remote C-DID
    pub relationships: HashMap<Rc<String>, Rc<Mutex<Relationship>>>,

    /*
    /// Mapping relationships by our R-DIDs
    pub r_map: HashMap<Rc<String>, Vec<HashSet<Rc<Relationship>>>>,
    */
    /// latest BIP32 path pointer to use for new keys
    pub path_pointer: u32,
}

impl Relationships {
    /// Prints Relationship status to the console
    pub fn status(&self, contacts: &Contacts, our_c_did: &Rc<String>) {
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
        self.print_relationships(contacts, our_c_did);
    }

    pub fn print_relationships(&self, contacts: &Contacts, our_c_did: &Rc<String>) {
        if self.relationships.is_empty() {
            println!("{}", style("No relationships exist").color256(CLI_ORANGE));
        } else {
            for r in self.relationships.values() {
                let r = r.lock().unwrap();
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

                if &r.our_did != our_c_did {
                    println!(
                        "    {}{}",
                        style("Using local r-did: ").color256(CLI_BLUE),
                        style(&r.our_did).color256(CLI_PURPLE)
                    );
                }

                if r.remote_did != r.remote_c_did {
                    println!(
                        "    {}{}",
                        style("Using remote r-did: ").color256(CLI_BLUE),
                        style(&r.remote_did).color256(CLI_PURPLE)
                    );
                }
                println!(
                    "    {}{}{}{}{}{}",
                    style("State: ").color256(CLI_BLUE),
                    style(&r.state).color256(CLI_GREEN),
                    style(" Created: ").color256(CLI_BLUE),
                    style(r.created).color256(CLI_GREEN),
                    style(" Task ID: ").color256(CLI_BLUE),
                    style(&r.task_id).color256(CLI_GREEN)
                );
                println!();
            }
        }
    }

    /// Removes a relationship by it's task_id
    pub fn remove_by_task_id(&mut self, id: &Rc<String>) -> Option<Rc<Mutex<Relationship>>> {
        if let Some(relationship) = self
            .relationships
            .values()
            .find(|f| f.lock().unwrap().task_id == *id)
            .cloned()
        {
            self.relationships
                .remove(&relationship.lock().unwrap().remote_did);
            Some(relationship)
        } else {
            None
        }
    }

    /// Gets a relationship using the remote C-DID key
    pub fn get(&self, c_did: &Rc<String>) -> Option<Rc<Mutex<Relationship>>> {
        self.relationships.get(c_did).cloned()
    }

    /// Finds a relationship by it's task ID
    pub fn find_by_task_id(&self, task_id: &Rc<String>) -> Option<Rc<Mutex<Relationship>>> {
        self.relationships
            .values()
            .find(|f| &f.lock().unwrap().task_id == task_id)
            .cloned()
    }

    /// Finds a relationship by it's remote DID (could be C-DID or R-DID)
    pub fn find_by_remote_did(&self, did: &Rc<String>) -> Option<Rc<Mutex<Relationship>>> {
        self.relationships
            .values()
            .find(|r| {
                let lock = r.lock().unwrap();
                lock.remote_did == *did || lock.remote_c_did == *did
            })
            .cloned()
    }

    /// Generates ATM Profiles for established relationships where the local r-did is different
    /// than the local c-did
    pub async fn generate_profiles(
        &self,
        tdk: &TDK,
        our_c_did: &Rc<String>,
        mediator: &str,
        bip32_root: &ExtendedSigningKey,
        key_info: &HashMap<String, KeyInfoConfig>,
    ) -> Result<HashMap<Rc<String>, Arc<ATMProfile>>> {
        let atm = tdk.atm.clone().unwrap();

        let mut profiles: HashMap<Rc<String>, Arc<ATMProfile>> = HashMap::new();

        for relationship in self.relationships.values() {
            let (our_did, state) = {
                let lock = relationship.lock().unwrap();
                (lock.our_did.clone(), lock.state.clone())
            };
            if state == RelationshipState::Established && &our_did != our_c_did {
                // Create an ATMProfile for this relationship
                let profile =
                    ATMProfile::new(&atm, None, our_did.to_string(), Some(mediator.to_string()))
                        .await?;
                profiles.insert(our_did.clone(), atm.profile_add(&profile, false).await?);

                // Generate secrets for this DID
                let secrets: Vec<Secret> = key_info
                    .iter()
                    .filter_map(|(k, v)| {
                        if k.starts_with(our_did.as_str()) {
                            if let KeySourceMaterial::Derived { path } = &v.path {
                                if let Some(kp) = match v.purpose {
                                    KeyTypes::RelationshipVerification => Some(KeyPurpose::Signing),
                                    KeyTypes::RelationshipEncryption => {
                                        Some(KeyPurpose::Encryption)
                                    }
                                    _ => None,
                                } {
                                    bip32_root.get_secret_from_path(path, kp).ok().map(|mut s| {
                                        s.id = k.clone();
                                        s
                                    })
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                tdk.get_shared_state()
                    .secrets_resolver
                    .insert_vec(&secrets)
                    .await;
            }
        }

        Ok(profiles)
    }

    /// Filters relationships and only returns those that are established
    pub fn get_established_relationships(&self) -> Vec<Rc<Mutex<Relationship>>> {
        self.relationships
            .values()
            .filter_map(|r| {
                let lock = r.lock().unwrap();
                if lock.state == RelationshipState::Established {
                    Some(r.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Relationship {
    /// Task ID that this relationship may be attached to
    pub task_id: Rc<String>,

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
    pub relationships: Vec<Rc<Mutex<Relationship>>>,
    pub path_pointer: u32,
}

impl From<Relationships> for RelationshipsShadow {
    fn from(value: Relationships) -> Self {
        let relationships = value
            .relationships
            .values()
            .cloned()
            .collect::<Vec<Rc<Mutex<Relationship>>>>();
        RelationshipsShadow {
            relationships,
            path_pointer: value.path_pointer,
        }
    }
}

impl From<RelationshipsShadow> for Relationships {
    fn from(value: RelationshipsShadow) -> Self {
        let mut relationships: HashMap<Rc<String>, Rc<Mutex<Relationship>>> = HashMap::new();
        //let mut r_map: HashMap<Rc<String>, Vec<HashSet<Rc<Relationship>>>> = HashMap::new();

        for relationship in value.relationships {
            let remote_did = relationship.lock().unwrap().remote_c_did.clone();
            relationships.insert(remote_did.clone(), relationship.clone());

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
                .print_relationships(&config.private.contacts, &config.public.community_did);
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
            let alias = if let Some(alias) = sub_args.get_one::<String>("alias") {
                alias.to_string()
            } else {
                println!(
                    "{}",
                    style("ERROR: Alias must be specified when requesting a Relationship!")
                        .color256(CLI_RED)
                );
                bail!("Missing alias argument!");
            };
            let reason = sub_args.get_one::<String>("reason");
            let generate_did = sub_args.get_flag("generate-did");

            create_send_request(
                &tdk,
                config,
                &respondent,
                alias,
                reason.map(|s| s.as_str()),
                generate_did,
            )
            .await?;

            config.save(profile)?;
        }
        Some(("ping", sub_args)) => {
            let remote_did = if let Some(did) = sub_args.get_one::<String>("remote") {
                did.to_string()
            } else {
                println!(
                    "{}",
                    style("ERROR: You must specify the remote alias or DID!").color256(CLI_RED)
                );
                bail!("Remote alias or DID is required");
            };

            remote_ping(&tdk, config, &remote_did).await?;

            config.save(profile)?;
        }
        Some(("remove", sub_args)) => {
            let remote_did = if let Some(did) = sub_args.get_one::<String>("remote") {
                did.to_string()
            } else {
                println!(
                    "{}",
                    style("ERROR: You must specify the remote alias or DID!").color256(CLI_RED)
                );
                bail!("Remote Alias or DID is required");
            };

            let Some(contact) = config.private.contacts.find_contact(&remote_did) else {
                println!(
                    "{}{}",
                    style("ERROR: Couldn't find a contact for: ").color256(CLI_RED),
                    style(remote_did).color256(CLI_ORANGE)
                );
                bail!("Couldn't find contact");
            };

            let relationship = if let Some(r) = config
                .private
                .relationships
                .find_by_remote_did(&contact.did)
            {
                r
            } else {
                println!(
                    "{} {}",
                    style("ERROR: No relationship found for remote DID/alias:").color256(CLI_RED),
                    style(remote_did).color256(CLI_ORANGE)
                );
                bail!("No relationship found for remote DID/alias");
            };

            let remote_c_did = {
                let lock = relationship.lock().unwrap();
                lock.remote_c_did.clone()
            };

            config
                .private
                .relationships
                .relationships
                .remove(&remote_c_did);

            println!(
                "{} {}",
                style("✅ Relationship with remote DID removed:").color256(CLI_GREEN),
                style(remote_c_did).color256(CLI_GREEN)
            );

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
    let mut e_secret = Secret::generate_x25519(
        None,
        Some(&ed25519_private_to_x25519_private_key(
            e_key.signing_key.as_bytes(),
        )),
    )?;

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

async fn remote_ping(tdk: &TDK, config: &mut Config, remote: &str) -> Result<()> {
    let atm = tdk.atm.clone().unwrap();
    let protocols = Protocols::new();

    let Some(contact) = config.private.contacts.find_contact(remote) else {
        println!(
            "{}{}",
            style("ERROR: Couldn't find a contact for: ").color256(CLI_RED),
            style(remote).color256(CLI_ORANGE)
        );
        bail!("Couldn't find contact for remote address");
    };

    // Find the relationship
    let relationship = if let Some(r) = config.private.relationships.get(&contact.did) {
        r
    } else {
        println!(
            "{} {}",
            style("ERROR: No relationship found for remote DID/alias:").color256(CLI_RED),
            style(remote).color256(CLI_ORANGE)
        );
        bail!("No relationship found for remote DID/alias");
    };

    let (our_did, remote_did) = {
        let lock = relationship.lock().unwrap();
        (lock.our_did.clone(), lock.remote_did.clone())
    };

    let profile = if our_did == config.public.community_did {
        &config.community_did.profile
    } else if let Some(profile) = config.atm_profiles.get(&our_did) {
        profile
    } else {
        println!(
            "{}{}",
            style("ERROR: Couldn't find Messaging profile for DID: ").color256(CLI_RED),
            style(&our_did).color256(CLI_ORANGE)
        );
        bail!("Missing Messaging Profile");
    };

    let ping_msg =
        protocols
            .trust_ping
            .generate_ping_message(Some(our_did.as_str()), &remote_did, true)?;
    let msg_id = ping_msg.id.clone();

    // Pack the message
    let (ping_msg, _) = ping_msg
        .pack_encrypted(
            &remote_did,
            Some(&our_did),
            Some(&our_did),
            tdk.did_resolver(),
            &tdk.get_shared_state().secrets_resolver,
            &PackEncryptedOptions {
                forward: false,
                ..Default::default()
            },
        )
        .await?;

    atm.forward_and_send_message(
        profile,
        false,
        &ping_msg,
        None,
        &config.public.mediator_did,
        &remote_did,
        None,
        None,
        false,
    )
    .await?;

    config.public.logs.insert(
        LogFamily::Relationship,
        format!(
            "Sent ping to remote DID: {} via local DID: {}",
            remote_did, our_did
        ),
    );

    config.private.tasks.new_task(
        &Rc::new(msg_id),
        TaskType::TrustPing {
            from: our_did,
            to: remote_did,
            relationship,
        },
    );

    println!("{}", style("✅ Ping Successfully sent... Run lkmv tasks interactive to check for pong response. NOTE: The remote recipient needs to check their messages first!").color256(CLI_GREEN));

    Ok(())
}
