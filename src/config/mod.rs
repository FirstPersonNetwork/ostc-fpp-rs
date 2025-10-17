/*! Contains the Lkmv CLI Tool Configuration
*
* Configuration is spread across three different contexts:
* 1. [Config]: Represents the active in-memory application config
* 2. [secured_config::SecuredConfig]: Represents [Config] info that is stored securely
* 3. [public_config::PublicConfig]: Represents [Config] info that is stored in plaintext on disk
*
* NOTE: Secure Config information is saved item by item as needed to the secure storage
*/

use crate::{
    config::{public_config::PublicConfig, secured_config::SecuredConfig},
    get_unlock_code,
};
use affinidi_tdk::did_common::Document;
use anyhow::{Context, Result};
use ed25519_dalek_bip32::ExtendedSigningKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod public_config;
pub mod secured_config;

/// Configuration information for lkmv tool
/// This is the active configuration used by the application itself
/// When you want to load/save this configuration, it will become:
/// 1. [public_config::PublicConfig]: Configuration information that is saved to disk
/// 2. [secured_config::SecuredConfig]: Configuration information that is encrypted and saved to secure storage
#[derive(Debug)]
pub struct Config {
    /// Root node of derivitave keys
    pub bip32_seed: ExtendedSigningKey,

    /// Our public Community DID used to identify ourselves within the Linux Foundation ecosystem
    pub community_did: CommunityDID,

    /// If using a hardware token, what is it's ID?
    pub token_id: Option<String>,

    /// If no hardware token, then should we use an unlock hash?
    pub unlock_code: bool,

    /// Where did the key values come from? Derived or Imported?
    pub keys_path: HashMap<String, KeySourceMaterial>,
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
    /// Sourced from BIP32 derivative, Path for this key
    Derived { path: String },

    /// Sourced from an external Key Import
    /// Key Material will be stored in the OS Secure Store
    Imported { key_id: String },
}

impl Config {
    /// Handles saving
    pub fn save(&self) -> Result<()> {
        let pc = PublicConfig::from(self);
        pc.save()?;

        Ok(())
    }

    pub fn load() -> Result<Self> {
        let pc = PublicConfig::load().context("Couldn't load Public Configuration")?;

        let unlock_code = if pc.token_id.is_none() && pc.unlock_code {
            Some(get_unlock_code()?)
        } else {
            None
        };

        let sc = SecuredConfig::load(pc.token_id.as_ref(), unlock_code.as_ref())?;

        todo!("Config::load() needs to be completed");
    }
}
