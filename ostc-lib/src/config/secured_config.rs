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
use crate::config::TokenInteractions;
use crate::{
    config::{Config, KeyTypes, UnlockCode},
    errors::OSTCError,
};
use aes_gcm::{AeadCore, Aes256Gcm, KeyInit, aead::Aead};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use keyring::Entry;
use rand::{SeedableRng, rngs::StdRng};
use secrecy::ExposeSecret;
#[cfg(feature = "openpgp-card")]
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, warn};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Constants for storing secure info in the OS Secure Store
const SERVICE: &str = "ostc";

/// Methods of protecting [SecuredConfig]
#[derive(Clone, Debug, Default)]
pub enum ProtectionMethod {
    TokenEncrypted,
    PasswordEncrypted,
    PlainText,
    #[default]
    Unknown,
}

impl From<SecuredConfigFormat> for ProtectionMethod {
    fn from(format: SecuredConfigFormat) -> Self {
        match format {
            SecuredConfigFormat::TokenEncrypted { .. } => ProtectionMethod::TokenEncrypted,
            SecuredConfigFormat::PasswordEncrypted { .. } => ProtectionMethod::PasswordEncrypted,
            SecuredConfigFormat::PlainText { .. } => ProtectionMethod::PlainText,
        }
    }
}

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
        #[cfg(feature = "openpgp-card")] user_pin: &SecretString,
        token: Option<&String>,
        unlock: Option<&UnlockCode>,
        #[cfg(feature = "openpgp-card")] touch_prompt: &impl TokenInteractions,
    ) -> Result<SecuredConfig, OSTCError> {
        let raw_bytes = match self {
            SecuredConfigFormat::TokenEncrypted { esk, data } => {
                // Token Encrypted format
                if let Some(token) = token {
                    #[cfg(feature = "openpgp-card")]
                    {
                        use crate::openpgp_card::crypt::token_decrypt;

                        token_decrypt(
                            #[cfg(feature = "openpgp-card")]
                            user_pin,
                            token,
                            &BASE64_URL_SAFE_NO_PAD.decode(esk)?,
                            &BASE64_URL_SAFE_NO_PAD.decode(data)?,
                            touch_prompt,
                        )?
                    }
                    #[cfg(not(feature = "openpgp-card"))]
                    {
                        warn!(
                            "Token has been configured, but no openpgp-card feature-flag has been enabled! exiting..."
                        );
                        return Err(OSTCError::Config("Token has been configured, but no openpgp-card feature-flag has been enabled! exiting.".to_string()));
                    }
                } else {
                    warn!(
                        "Secured Config is Token Encrypted, but no token identifier has been provided!"
                    );
                    return Err(OSTCError::Config("Secured Config is Token Encrypted, but no token identifier has been provided!".to_string()));
                }
            }
            SecuredConfigFormat::PasswordEncrypted { data } => {
                // Password Encrypted format
                if let Some(unlock) = unlock {
                    unlock_code_decrypt(
                        unlock.0.expose_secret().first_chunk::<32>().unwrap(),
                        &BASE64_URL_SAFE_NO_PAD.decode(data)?,
                    )
                    .map_err(|e| {
                        OSTCError::Decrypt(format!(
                            "Couldn't decrypt password encrypted SecuredConfig. Reason: {e}"
                        ))
                    })?
                } else {
                    return Err(OSTCError::Config(
                        "Secured Config is Password Encrypted, but no unlock code has been provided!".to_string()
                    ));
                }
            }
            SecuredConfigFormat::PlainText { text } => {
                // Plaintext format - no checks needed

                BASE64_URL_SAFE_NO_PAD.decode(text)?
            }
        };

        Ok(serde_json::from_slice(raw_bytes.as_slice())?)
    }
}

/// Secured Configuration information for ostc tool
/// Try to keep this as small as possible for ease of secure storage
#[derive(Serialize, Deserialize, Debug, Zeroize, ZeroizeOnDrop)]
pub struct SecuredConfig {
    // base64 encoded BIP32 private seed
    pub bip32_seed: String,

    /// Key information containing path info
    /// key is the DID VerificationMethod ID
    #[zeroize(skip)] // chrono doesn't support zeroize
    pub key_info: HashMap<String, KeyInfoConfig>,

    #[serde(skip, default)]
    #[zeroize(skip)]
    pub protection_method: ProtectionMethod,
}

impl From<&Config> for SecuredConfig {
    /// Extracts secured/private information from the full Config
    fn from(cfg: &Config) -> Self {
        SecuredConfig {
            bip32_seed: cfg.bip32_seed.expose_secret().to_owned(),
            key_info: cfg.key_info.clone(),
            protection_method: cfg.protection_method.clone(),
        }
    }
}

impl SecuredConfig {
    /// Internal private function that saves a SecuredConfig to the OS Secure Store
    /// Encrypts the secret info as needed based on token/unlock parameters
    /// Converts to BASE64 then saves to OS Secure Store
    pub fn save(
        &self,
        profile: &str,
        token: Option<&String>,
        unlock: Option<&Vec<u8>>,
        #[cfg(feature = "openpgp-card")] touch_prompt: &(dyn Fn() + Send + Sync),
    ) -> Result<(), OSTCError> {
        let entry = Entry::new(SERVICE, profile).map_err(|e| {
            OSTCError::Config(format!(
                "Couldn't open OS Secure Store for profile ({profile}). Reason: {e}"
            ))
        })?;

        // Serialize SecuredConfig to byte array
        let input = serde_json::to_vec(&self)?;

        let formatted = if let Some(token) = token {
            #[cfg(feature = "openpgp-card")]
            {
                use crate::openpgp_card::crypt::token_encrypt;

                let (esk, data) = token_encrypt(token, &input, touch_prompt)?;
                SecuredConfigFormat::TokenEncrypted {
                    esk: BASE64_URL_SAFE_NO_PAD.encode(&esk),
                    data: BASE64_URL_SAFE_NO_PAD.encode(&data),
                }
            }
            #[cfg(not(feature = "openpgp-card"))]
            return Err(OSTCError::Config( "Token has been configured, but no openpgp-card feature-flag has been enabled! exiting...".to_string()));
        } else if let Some(unlock) = unlock {
            SecuredConfigFormat::PasswordEncrypted {
                data: BASE64_URL_SAFE_NO_PAD.encode(unlock_code_encrypt(
                    unlock.first_chunk::<32>().unwrap(),
                    &input,
                )?),
            }
        } else {
            // Plain-text
            SecuredConfigFormat::PlainText {
                text: BASE64_URL_SAFE_NO_PAD.encode(input),
            }
        };

        // Save this to the OS Secure Store
        entry
            .set_secret(serde_json::to_string_pretty(&formatted)?.as_bytes())
            .map_err(|e| {
                OSTCError::Config(format!(
                    "Couldn't save encrypted config to the OS Secure Store. Reason: {e}"
                ))
            })?;
        Ok(())
    }

    /// Loads secret info from the OS Secure Store
    /// token: Hardware token identifier if being used
    /// unlock: Use a Password/PIN to unlock secret storage if no hardware token
    /// If token is None and unlock is false, assumes no protection apart from the OS Secure Store
    /// itself
    pub fn load(
        profile: &str,
        #[cfg(feature = "openpgp-card")] user_pin: &SecretString,
        token: Option<&String>,
        unlock: Option<&UnlockCode>,
        #[cfg(feature = "openpgp-card")] touch_prompt: &impl TokenInteractions,
    ) -> Result<Self, OSTCError> {
        let entry = Entry::new(SERVICE, profile).map_err(|e| {
            OSTCError::Config(format!(
                "Couldn't access OS Secure Store for profile ({profile}). Reason: {e}",
            ))
        })?;

        let raw_secured_config: SecuredConfigFormat = match entry.get_secret() {
            Ok(secret) => match serde_json::from_slice(secret.as_slice()) {
                Ok(format) => format,
                Err(e) => {
                    error!(
                        "ERROR: Format of SecuredConfig in OS Secure store is invalid! Reason: {e}"
                    );
                    return Err(OSTCError::Config(format!(
                        "Couldn't load ostc secured configuration. Reason: {e}"
                    )));
                }
            },
            Err(e) => {
                error!("Couldn't find Secure Config in the OS Secret Store. Fatal Error: {e}");
                return Err(OSTCError::Config(format!(
                    "Couldn't find ostc secured configuration. Reason: {e}"
                )));
            }
        };

        raw_secured_config.unlock(
            #[cfg(feature = "openpgp-card")]
            user_pin,
            token,
            unlock,
            #[cfg(feature = "openpgp-card")]
            touch_prompt,
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

    #[zeroize(skip)]
    #[serde(default)]
    pub purpose: KeyTypes,
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
pub fn unlock_code_encrypt(unlock: &[u8; 32], input: &[u8]) -> Result<Vec<u8>, OSTCError> {
    let mut rng = StdRng::from_seed(*unlock);
    let key = Aes256Gcm::generate_key(&mut rng);
    let nonce = Aes256Gcm::generate_nonce(&mut rng);
    let cipher = Aes256Gcm::new(&key);

    match cipher.encrypt(&nonce, input) {
        Ok(encrypted) => Ok(encrypted),
        Err(e) => {
            error!("Couldn't encrypt data. Reason: {e}");
            Err(OSTCError::Encrypt(format!(
                "Couldn't encrypt data. Reason: {e}"
            )))
        }
    }
}

/// Creates an AES-256 key from the hash of the unlock code and attempts to decrypt using it
pub fn unlock_code_decrypt(unlock: &[u8; 32], input: &[u8]) -> Result<Vec<u8>, OSTCError> {
    let mut rng = StdRng::from_seed(*unlock);
    let key = Aes256Gcm::generate_key(&mut rng);
    let nonce = Aes256Gcm::generate_nonce(&mut rng);
    let cipher = Aes256Gcm::new(&key);

    match cipher.decrypt(&nonce, input) {
        Ok(decrypted) => Ok(decrypted),
        Err(e) => {
            error!("Couldn't decrypt data. Likely due to incorrect unlock code! Reason: {e}");
            Err(OSTCError::Decrypt(format!(
                "Couldn't decrypt data, likely due to incorrect unlock code! Reason: {e}"
            )))
        }
    }
}
