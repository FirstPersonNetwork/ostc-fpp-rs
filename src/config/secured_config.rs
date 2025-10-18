/*!
*  Secured [crate::config::Config] information that is stored in the OS Secure Storage
*
*  * If using hardware tokens, then the data is encrypted/decrypted using the hardware token
*  * If no hardware token, then may be using a passphrase to protect the data
*  * If no hardware token, and no passphrase, then is in plaintext in the OS Secure Store
*
*  Must intially save bip32_seed first before any keys can be stored
*/

use crate::{CLI_ORANGE, CLI_RED, config::KeySourceMaterial};
use aes_gcm::{AeadCore, Aes256Gcm, KeyInit, aead::Aead};
use affinidi_tdk::secrets_resolver::secrets::Secret;
use anyhow::{Context, Result, bail};
use base64::{
    Engine,
    prelude::{BASE64_STANDARD_NO_PAD, BASE64_URL_SAFE_NO_PAD},
};
use console::style;
use keyring::Entry;
use rand::{SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Constants for storing secure info in the OS Secure Store
const SERVICE: &str = "lkmv";
const USER: &str = "lkmv-secrets";

/// Secured Configuration information for lkmv tool
/// Try to keep this as small as possible for ease of secure storage
#[derive(Serialize, Deserialize, Debug)]
pub struct SecuredConfig {
    // base64 encoded BIP32 private seed
    pub bip32_seed: String,

    /// Secrets stored in the OS Secure Storage
    /// key: #key-id
    /// value: Secret
    pub keys: HashMap<String, Secret>,

    /// Where did the keys being used come from?
    /// key: #key-id
    /// value: Derived Path (BIP32 or Imported)
    pub keys_path: HashMap<String, KeySourceMaterial>,
}

impl SecuredConfig {
    /// Create a blank new SecuredConfig with just the BIP32 seed
    pub fn new(bip32: &[u8]) -> Self {
        let bip32 = BASE64_URL_SAFE_NO_PAD.encode(bip32);
        SecuredConfig {
            bip32_seed: bip32,
            keys: HashMap::new(),
            keys_path: HashMap::new(),
        }
    }

    /// Does a fresh save of a SecuredConfig to the OS Secure Store
    pub fn initial_save(&self, token: Option<&String>, unlock: Option<&[u8; 32]>) -> Result<()> {
        self.save(token, unlock)?;
        Ok(())
    }

    /// Internal private function that saves a SecuredConfig to the OS Secure Store
    /// Encrypts the secret info as needed based on token/unlock parameters
    /// Converts to BASE64 then saves to OS Secure Store
    fn save(&self, token: Option<&String>, unlock: Option<&[u8; 32]>) -> Result<()> {
        let entry = Entry::new(SERVICE, USER)?;

        // Serialize SecuredConfig to byte array
        let input =
            serde_json::to_vec(&self).context("Couldn't serialize Secured Configuration")?;

        let bytes = if let Some(token) = token {
            #[cfg(feature = "openpgp-card")]
            {
                use crate::openpgp_card::crypt::token_encrypt;

                token_encrypt(token, &input)?
            }
            #[cfg(not(feature = "openpgp-card"))]
            bail!(
                "Token has been configured, but no openpgp-card feature-flag has been enabled! exiting..."
            );
        } else if let Some(unlock) = unlock {
            println!("TIMTAM: Using passphrase");
            unlock_code_encrypt(unlock, &input)?
        } else {
            // Plain-text
            input
        };

        entry.set_secret(BASE64_URL_SAFE_NO_PAD.encode(bytes).as_bytes())?;
        Ok(())
    }

    /// Loads secret info from the OS Secure Store
    /// token: Hardware token identifier if being used
    /// unlock: Use a Password/PIN to unlock secret storage if no hardware token
    /// If token is None and unlock is false, assumes no protection apart from the OS Secure Store
    /// itself
    pub fn load(token: Option<&String>, unlock: Option<&[u8; 32]>) -> Result<Self> {
        let entry = Entry::new(SERVICE, USER)?;
        let secret_bytes =
            match entry.get_secret() {
                Ok(secret) => BASE64_URL_SAFE_NO_PAD
                    .decode(secret)
                    .context("Couldn't decode BASE64 Secure Config from OS Secret Store")?,
                Err(keyring::error::Error::NoEntry) => {
                    bail!(keyring::error::Error::NoEntry);
                }
                Err(e) => {
                    println!(
                "{}{}",
                style("ERROR: Couldn't find Secure Config in the OS Secret Store. Fatal Error: ")
                    .color256(CLI_RED),
                    style(e).color256(CLI_ORANGE)
            );
                    bail!("Couldn't load lkmv secured configuration");
                }
            };

        let plain_bytes = if let Some(token) = token {
            #[cfg(feature = "openpgp-card")]
            {
                "TODO: Token".to_string();
                secret_bytes
            }
            #[cfg(not(feature = "openpgp-card"))]
            bail!(
                "Token has been configured, but no openpgp-card feature-flag has been enabled! exiting..."
            );
        } else if let Some(unlock) = unlock {
            unlock_code_decrypt(unlock, &secret_bytes)?
        } else {
            // This is a raw string from the OS Secure Store
            secret_bytes
        };

        serde_json::from_slice(plain_bytes.as_slice())
            .context("Couldn't deserialize Secured Configuration")
    }

    /*
    pub fn save_key() -> Result<()> {}

        let mut sc = match SecuredConfig::load(token, unlock) {
            Ok(sc) => sc,
            Err(e) => {
                match e.downcast_ref::<keyring::error::Error>() {
                    Some(keyring::error::Error::NoEntry) => {
                        // No existing entry, create a new one
                        SecuredConfig {
                            bip32_seed: String::new(),
                            keys: HashMap::new(),
                            keys_path: HashMap::new(),
                        }
                    }
                    _ => {
                        println!(
                            "{}{}",
                            style("ERROR: There is an error with the OS Secure Store: ")
                                .color256(CLI_RED),
                            style(&e).color256(CLI_ORANGE)
                        );
                        bail!(e);
                    }
                }
            }
        };


    pub fn save_keys_path() -> Result<()> {}
    */
}

/// Creates an AES-256 key from the hash of the unlock code and attempts to encrypt using it
fn unlock_code_encrypt(unlock: &[u8; 32], input: &[u8]) -> Result<Vec<u8>> {
    let mut rng = StdRng::from_seed(*unlock);
    let key = Aes256Gcm::generate_key(&mut rng);
    let nonce = Aes256Gcm::generate_nonce(&mut rng);
    let cipher = Aes256Gcm::new(&key);

    match cipher.encrypt(&nonce, input) {
        Ok(encrypted) => Ok(encrypted),
        Err(e) => {
            println!(
                "{}{}",
                style("ERROR: Couldn't encrypt data. Reason: ").color256(CLI_RED),
                style(e).color256(CLI_ORANGE)
            );
            bail!(e);
        }
    }
}

/// Creates an AES-256 key from the hash of the unlock code and attempts to decrypt using it
fn unlock_code_decrypt(unlock: &[u8; 32], input: &[u8]) -> Result<Vec<u8>> {
    let mut rng = StdRng::from_seed(*unlock);
    let key = Aes256Gcm::generate_key(&mut rng);
    let nonce = Aes256Gcm::generate_nonce(&mut rng);
    let cipher = Aes256Gcm::new(&key);

    match cipher.decrypt(&nonce, input) {
        Ok(decrypted) => Ok(decrypted),
        Err(e) => {
            println!(
                "{}{}",
                style("ERROR: Couldn't decrypt data. Reason: ").color256(CLI_RED),
                style(e).color256(CLI_ORANGE)
            );
            bail!(e);
        }
    }
}
