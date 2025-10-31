/*! Contains the Lkmv CLI Tool Configuration
*
* Configuration is spread across three different contexts:
* 1. [Config]: Represents the active in-memory application config
* 2. [secured_config::SecuredConfig]: Represents [Config] info that is stored securely
* 3. [public_config::PublicConfig]: Represents [Config] info that is stored in plaintext on disk
*
* NOTE: Secure Config information is saved item by item as needed to the secure storage
*/

#[cfg(feature = "openpgp-card")]
use crate::openpgp_card::ui::{AdminPin, UserPin};
use crate::{
    config::{
        public_config::PublicConfig,
        secured_config::{KeySourceMaterial, SecuredConfig},
    },
    get_unlock_code,
    setup::CommunityDIDKeys,
};
use affinidi_tdk::{TDK, did_common::Document};
use anyhow::{Context, Result};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use console::Term;
use ed25519_dalek_bip32::ExtendedSigningKey;
use secrecy::SecretString;
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
    /// Public readable config items when saved to disk
    pub public: PublicConfig,

    /// Root node of derivative keys
    pub bip32_root: ExtendedSigningKey,

    // Protected BIP32 seed
    pub bip32_seed: SecretString,

    /// Where did the key values come from? Derived or Imported?
    pub keys_path: HashMap<String, KeySourceMaterial>,

    /// Community DID Secrets
    /// This is derived from SecuredConfig information
    pub community_did: CommunityDID,

    // *********************************************
    // Temporary Config values
    //
    #[cfg(feature = "openpgp-card")]
    /// Hardware token Admin PIN
    pub token_admin_pin: AdminPin,

    #[cfg(feature = "openpgp-card")]
    /// Hardware token User PIN
    pub token_user_pin: UserPin,
}

/// Our public Community DID used to identify ourselves within the Linux Foundation ecosystem
#[derive(Serialize, Deserialize, Debug)]
pub struct CommunityDID {
    /// DID Identifier String
    pub id: String,

    /// Resolved DID Document for this DID
    pub document: Document,

    /// Keys that represent the Community DID
    #[serde(skip)]
    pub keys: Option<CommunityDIDKeys>,
}

impl Config {
    /// Handles saving
    pub fn save(&self, unlock_code: Option<&[u8; 32]>) -> Result<()> {
        let pc = PublicConfig::from(self);
        pc.save()?;

        let sc = SecuredConfig::from(self);
        sc.save(self.public.token_id.as_ref(), unlock_code)?;

        Ok(())
    }

    pub async fn load(term: &Term, tdk: &mut TDK) -> Result<Self> {
        let pc = PublicConfig::load().context("Couldn't load Public Configuration")?;

        let unlock_code = if pc.token_id.is_none() && pc.unlock_code {
            Some(get_unlock_code()?)
        } else {
            None
        };

        #[cfg(feature = "openpgp-card")]
        let mut token_user_pin = UserPin::default();
        let sc = SecuredConfig::load(
            term,
            #[cfg(feature = "openpgp-card")]
            &mut token_user_pin,
            pc.token_id.as_ref(),
            unlock_code.as_ref(),
        )?;

        // All config info has been loaded, load DID Document and regenerate keys
        let rr = tdk
            .did_resolver()
            .resolve(&pc.community_did)
            .await
            .context("Couldn't resolve Community DID")?;
        let c_keys = Config::regenerate_community_keys(&sc, &pc, &rr.doc)?;

        Ok(Config {
            bip32_root: ExtendedSigningKey::from_seed(
                BASE64_URL_SAFE_NO_PAD
                    .decode(&sc.bip32_seed)
                    .context("Couldn't base64 decode BIP32 seed")?
                    .as_slice(),
            )?,
            community_did: CommunityDID {
                id: pc.community_did.clone(),
                document: rr.doc,
                keys: Some(c_keys),
            },
            bip32_seed: SecretString::new(sc.bip32_seed),
            public: pc,
            keys_path: sc.keys_path,
            #[cfg(feature = "openpgp-card")]
            token_admin_pin: AdminPin::default(),
            #[cfg(feature = "openpgp-card")]
            token_user_pin,
        })
    }

    /// Private function that regenerates the Community DID keys from secured config
    fn regenerate_community_keys(
        sc: &SecuredConfig,
        pc: &PublicConfig,
        doc: &Document,
    ) -> Result<CommunityDIDKeys> {
        todo!("Regenerate Community DID Keys");
    }
}
