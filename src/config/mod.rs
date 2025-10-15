/*! Contains the Lkmv CLI Tool Configuration
*
* Configuration is spread across three different contexts:
* 1. [Config]: Represents the active in-memory application config
* 2. [secured_config::SecuredConfig]: Represents [Config] info that is stord securely
* 3. [public_config::PublicConfig]: Represents [Config] info that is stored in plaintext on disk
*/

use affinidi_tdk::did_common::Document;
use serde::{Deserialize, Serialize};

pub mod public_config;
pub mod secured_config;

/// Configuration information for lkmv tool
/// This is the active Configuration used by the application itself
/// When you want to load/save this configuration, it will become:
/// 1. [public_config::PublicConfig]: Configuration information that is saved to disk
/// 2. [secured_config::SecuredConfig]: Configuration information that is encrypted and saved to secure storage
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    /// Our public Community DID used to identify ourselves within the Linux Foundation ecosystem
    pub community_did: CommunityDID,

    pub keys: Vec<KeySourceMaterial>,
}

/// Our public Community DID used to identify ourselves within the Linux Foundation ecosystem
#[derive(Serialize, Deserialize, Debug)]
pub struct CommunityDID {
    /// DID Identifier String
    pub id: String,

    /// Resolved DID Document for this DID
    pub document: Document,
}
/// Where did the source for the Key Material come from?
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum KeySourceMaterial {
    /// Sourced from BIP32 derivitive, Path for this key
    Derived { path: String },

    /// Sourced from an external Key Import
    /// Key Material will be stored in the OS Secure Store
    Imported { key_id: String },
}
