/*!
*  Secured [crate::config::Config] information that is stored in the OS Secure Storage
*
*  * If using hardware tokens, then the data is encrypted/decrypted using the hardware token
*  * If no hardware token, then may be using a passphrase to protect the data
*  * If no hardware token, and no passphrase, then is in plaintext in the OS Secure Store
*
*  Must intially save bip32_seed first before any keys can be stored
*/

#[cfg(feature = "openpgp-card")]
use crate::openpgp_card::ui::UserPin;
use crate::{CLI_ORANGE, CLI_RED, config::Config};
use aes_gcm::{AeadCore, Aes256Gcm, KeyInit, aead::Aead};
use anyhow::{Context, Result, bail};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use console::{Term, style};
use keyring::Entry;
use rand::{SeedableRng, rngs::StdRng};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Constants for storing secure info in the OS Secure Store
const SERVICE: &str = "lkmv";
const USER: &str = "lkmv-secrets";

/// Three possible formats to store [SecuredConfig]
/// 1. TokenEncrypted - Encrypted using a hardware token
/// 2. PasswordEncrypted - Encrypted from a derived key from a password/PIN
/// 3. PlainText - No Encryption at all - USE AT YOUR OWN RISK!
///
/// NOTE: All strings are BASE64 encoded
#[derive(Serialize, Deserialize, Debug, Zeroize)]
#[serde(untagged)]
enum SecuredConfigFormat {
    /// Hardware token encrypted data
    TokenEncrypted {
        /// Encrypted Session Key
        esk: String,
        /// Encrypted data using esk
        data: String,
    },

    /// Password/PIN Protected data
    PasswordEncrypted {
        /// Encrypted data using AES-256 from derived key
        data: String,
    },

    /// Plaintext data - dangerous!
    PlainText {
        /// Plaintext data that can be Serialized into [SecuredConfig]
        text: String,
    },
}

impl SecuredConfigFormat {
    /// Loads secret info from the OS Secure Store
    pub fn unlock(
        &self,
        term: &Term,
        #[cfg(feature = "openpgp-card")] user_pin: &mut UserPin,
        token: Option<&String>,
        unlock: Option<&[u8; 32]>,
    ) -> Result<SecuredConfig> {
        let raw_bytes = match self {
            SecuredConfigFormat::TokenEncrypted { esk, data } => {
                // Token Encrypted format
                if let Some(token) = token {
                    #[cfg(feature = "openpgp-card")]
                    {
                        use crate::openpgp_card::crypt::token_decrypt;

                        token_decrypt(
                            term,
                            #[cfg(feature = "openpgp-card")]
                            user_pin,
                            token,
                            &BASE64_URL_SAFE_NO_PAD
                                .decode(esk)
                                .context("Couldn't base64 decode TokenEncrypted ESK")?,
                            &BASE64_URL_SAFE_NO_PAD
                                .decode(data)
                                .context("Couldn't base64 decode TokenEncrypted SecuredConfig")?,
                        )?
                    }
                    #[cfg(not(feature = "openpgp-card"))]
                    bail!(
                        "Token has been configured, but no openpgp-card feature-flag has been enabled! exiting..."
                    );
                } else {
                    bail!(
                        "Secured Config is Token Encrypted, but no token identifier has been provided!"
                    );
                }
            }
            SecuredConfigFormat::PasswordEncrypted { data } => {
                // Password Encrypted format
                if let Some(unlock) = unlock {
                    unlock_code_decrypt(
                        unlock,
                        &BASE64_URL_SAFE_NO_PAD
                            .decode(data)
                            .context("Couldn't base64 decode password encrypted SecuredConfig")?,
                    )
                    .context("Couldn't decrypt password encrypted SecuredConfig")?
                } else {
                    bail!(
                        "Secured Config is Password Encrypted, but no unlock code has been provided!"
                    );
                }
            }
            SecuredConfigFormat::PlainText { text } => {
                // Plaintext format - no checks needed

                BASE64_URL_SAFE_NO_PAD
                    .decode(text)
                    .context("Couldn't base64 decode plaintext SecuredConfig")?
            }
        };

        serde_json::from_slice(raw_bytes.as_slice()).context("Couldn't deserialize SecuredConfig")
    }
}

/// Secured Configuration information for lkmv tool
/// Try to keep this as small as possible for ease of secure storage
#[derive(Serialize, Deserialize, Debug, Zeroize, ZeroizeOnDrop)]
pub struct SecuredConfig {
    // base64 encoded BIP32 private seed
    pub bip32_seed: String,

    #[zeroize(skip)] // chrono doesn't support zeroize
    pub key_info: HashMap<String, KeyInfoConfig>,
}

impl From<&Config> for SecuredConfig {
    /// Extracts secured/private information from the full Config
    fn from(cfg: &Config) -> Self {
        SecuredConfig {
            bip32_seed: cfg.bip32_seed.expose_secret().to_owned(),
            key_info: cfg.key_info.clone(),
        }
    }
}

impl SecuredConfig {
    /// Internal private function that saves a SecuredConfig to the OS Secure Store
    /// Encrypts the secret info as needed based on token/unlock parameters
    /// Converts to BASE64 then saves to OS Secure Store
    pub fn save(&self, token: Option<&String>, unlock: Option<&[u8; 32]>) -> Result<()> {
        let entry = Entry::new(SERVICE, USER)?;

        // Serialize SecuredConfig to byte array
        let input =
            serde_json::to_vec(&self).context("Couldn't serialize Secured Configuration")?;

        let formatted = if let Some(token) = token {
            #[cfg(feature = "openpgp-card")]
            {
                use crate::openpgp_card::crypt::token_encrypt;

                let (esk, data) = token_encrypt(token, &input)?;
                SecuredConfigFormat::TokenEncrypted {
                    esk: BASE64_URL_SAFE_NO_PAD.encode(&esk),
                    data: BASE64_URL_SAFE_NO_PAD.encode(&data),
                }
            }
            #[cfg(not(feature = "openpgp-card"))]
            bail!(
                "Token has been configured, but no openpgp-card feature-flag has been enabled! exiting..."
            );
        } else if let Some(unlock) = unlock {
            SecuredConfigFormat::PasswordEncrypted {
                data: BASE64_URL_SAFE_NO_PAD.encode(
                    unlock_code_encrypt(unlock, &input)
                        .context("Couldn't encrypt SecuredConfig")?,
                ),
            }
        } else {
            // Plain-text
            SecuredConfigFormat::PlainText {
                text: BASE64_URL_SAFE_NO_PAD.encode(input),
            }
        };

        // Save this to the OS Secure Store
        entry.set_secret(
            serde_json::to_string_pretty(&formatted)
                .context("Couldn't serialize SecuredConfigFormat")?
                .as_bytes(),
        )?;
        Ok(())
    }

    /// Loads secret info from the OS Secure Store
    /// token: Hardware token identifier if being used
    /// unlock: Use a Password/PIN to unlock secret storage if no hardware token
    /// If token is None and unlock is false, assumes no protection apart from the OS Secure Store
    /// itself
    pub fn load(
        term: &Term,
        #[cfg(feature = "openpgp-card")] user_pin: &mut UserPin,
        token: Option<&String>,
        unlock: Option<&[u8; 32]>,
    ) -> Result<Self> {
        let entry = Entry::new(SERVICE, USER)?;
        let raw_secured_config: SecuredConfigFormat =
            match entry.get_secret() {
                Ok(secret) => match serde_json::from_slice(secret.as_slice()) {
                    Ok(format) => format,
                    Err(e) => {
                        println!(
                "{}{}",
                style("ERROR: Format of SecuredConfig in OS Secure store is invalid! Reason: ")
                    .color256(CLI_RED),
                    style(e).color256(CLI_ORANGE)
            );
                        bail!("Couldn't load lkmv secured configuration");
                    }
                },
                Err(e) => {
                    println!(
                "{}{}",
                style("ERROR: Couldn't find Secure Config in the OS Secret Store. Fatal Error: ")
                    .color256(CLI_RED),
                    style(e).color256(CLI_ORANGE)
            );
                    bail!("Couldn't find lkmv secured configuration");
                }
            };

        raw_secured_config.unlock(
            term,
            #[cfg(feature = "openpgp-card")]
            user_pin,
            token,
            unlock,
        )
    }
}

/// Information that is required for each key stored
#[derive(Clone, Serialize, Deserialize, Debug, Zeroize, ZeroizeOnDrop)]
pub struct KeyInfoConfig {
    /// Where did the keys being used come from?
    /// key: #key-id
    /// value: Derived Path (BIP32 or Imported)
    pub path: KeySourceMaterial,

    /// When wss this key first created?
    #[zeroize(skip)] // chrono doesn't support zeroize
    pub create_time: DateTime<Utc>,
}
/// Where did the source for the Key Material come from?
#[derive(Clone, Serialize, Deserialize, Debug, Zeroize, ZeroizeOnDrop)]
pub enum KeySourceMaterial {
    /// Sourced from BIP32 derivative, Path for this key
    Derived { path: String },

    /// Sourced from an external Key Import
    /// multiencoded private key
    /// Key Material will be stored in the OS Secure Store
    Imported { seed: String },
}

/// Creates an AES-256 key from the hash of the unlock code and attempts to encrypt using it
pub fn unlock_code_encrypt(unlock: &[u8; 32], input: &[u8]) -> Result<Vec<u8>> {
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
pub fn unlock_code_decrypt(unlock: &[u8; 32], input: &[u8]) -> Result<Vec<u8>> {
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
            println!(
                "  {}",
                style("Likely due to using an incorrect unlock code!").color256(CLI_ORANGE)
            );
            bail!(e);
        }
    }
}
