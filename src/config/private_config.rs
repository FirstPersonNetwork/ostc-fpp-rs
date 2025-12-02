/*! Configuration information that is required that should be treated as private/sensitive
*   but is not as critical as private key information which is stored in the OS Secure Store
*/

use crate::{
    CLI_ORANGE, CLI_RED,
    config::secured_config::{unlock_code_decrypt, unlock_code_encrypt},
    contacts::Contacts,
    relationships::Relationships,
    tasks::Tasks,
    vrc::Vrcs,
};
use anyhow::Result;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use console::style;
use ed25519_dalek_bip32::{DerivationPath, ExtendedSigningKey};
use secrecy::{ExposeSecret, SecretVec};
use serde::{Deserialize, Serialize};

/// Primary structure used for storing private [crate::config::Config] data that is sensitive but
/// not key data
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct PrivateConfig {
    /// Known contacts and associated information
    pub contacts: Contacts,

    /// Relationships information
    #[serde(default)]
    pub relationships: Relationships,

    /// Known Tasks
    #[serde(default)]
    pub tasks: Tasks,

    /// VRCs Issued
    /// key = remote C-DID
    pub vrcs_issued: Vrcs,

    /// VRCs received
    /// key = remote C-DID
    pub vrcs_received: Vrcs,
}

impl PrivateConfig {
    /// Converts PrivateConfig to an encrypted BASE64 string for saving to disk
    pub fn save(&self, seed_bytes: &SecretVec<u8>) -> Result<String> {
        let bytes = serde_json::to_vec(self)?;

        let secured = match unlock_code_encrypt(
            seed_bytes.expose_secret().first_chunk::<32>().unwrap(),
            &bytes,
        ) {
            Ok(result) => result,
            Err(e) => {
                println!(
                    "{}{}",
                    style("ERROR: Couldn't encrypt settings. Reason: ").color256(CLI_RED),
                    style(&e).color256(CLI_ORANGE)
                );
                return Err(e);
            }
        };

        Ok(BASE64_URL_SAFE_NO_PAD.encode(&secured))
    }

    pub fn load(seed_bytes: &SecretVec<u8>, input: &str) -> Result<PrivateConfig> {
        let bytes = BASE64_URL_SAFE_NO_PAD.decode(input)?;

        let bytes = unlock_code_decrypt(
            seed_bytes.expose_secret().first_chunk::<32>().unwrap(),
            &bytes,
        )?;

        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn get_seed(bip32: &ExtendedSigningKey, path: &str) -> Result<SecretVec<u8>> {
        Ok(SecretVec::new(
            bip32
                .derive(&path.parse::<DerivationPath>()?)?
                .verifying_key()
                .to_bytes()
                .to_vec(),
        ))
    }
}
