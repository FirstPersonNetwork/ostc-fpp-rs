/*! Contains the Lkmv CLI Tool Configuration
*
* Configuration is spread across four different contexts:
* 1. [Config]: Represents the active in-memory application config
* 2. [secured_config::SecuredConfig]: Represents [Config] info that is stored securely (key info)
* 3. [public_config::PublicConfig]: Represents [Config] info that is stored in plaintext on disk
* 4. [protected_config::ProtectedConfig]: Represents [Config] info that is encryoted and stored on disk
*
* NOTE: Secure Config information is saved item by item as needed to the secure storage
*/

use crate::logs::LogFamily;
use crate::{
    KeyPurpose,
    bip32::Bip32Extension,
    config::{
        protected_config::ProtectedConfig,
        public_config::PublicConfig,
        secured_config::{
            KeyInfoConfig, KeySourceMaterial, ProtectionMethod, SecuredConfig, unlock_code_encrypt,
        },
    },
    errors::LKMVError,
};
use affinidi_tdk::{
    TDK,
    did_common::{Document, document::DocumentExt},
    messaging::profiles::ATMProfile,
    secrets_resolver::{SecretsResolver, secrets::Secret},
};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
use dialoguer::{Password, theme::ColorfulTheme};
use dtg_credentials::DTGCredential;
use ed25519_dalek_bip32::ExtendedSigningKey;
use secrecy::{ExposeSecret, SecretString, SecretVec};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fmt::Display, fs, rc::Rc, sync::Arc};
use tracing::warn;

pub mod did;
pub mod protected_config;
pub mod public_config;
pub mod secured_config;

/// Is always a SHA2-256 hash of a user provided passphrase
pub struct UnlockCode(SecretVec<u8>);

impl UnlockCode {
    pub fn from_string(s: &str) -> Self {
        let hash = Sha256::digest(s.as_bytes());
        UnlockCode(SecretVec::new(hash.to_vec()))
    }
}

/// How is the config protected?
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub enum ConfigProtectionType {
    /// Requires a hardware token with the Token ID to unlock config
    /// Will need to provide the USER PIN to the token
    Token(String),

    /// Requires an unlock passphrase to unlock config
    /// Will need to provide the unlock passphrase
    #[default]
    Encrypted,

    /// Is not encrypted in any way
    Plaintext,
}

#[cfg(feature = "openpgp-card")]
/// If the token requires interaction then these methods help with user interaction
/// touch_notify() is called before the token may require touch
/// touch_completed() is called after the token operation has been completed
pub trait TokenInteractions: Send + Sync {
    /// Notifies application that a token touch is required
    fn touch_notify(&self);

    /// Notifies application that token has completed it's operation
    fn touch_completed(&self);
}

/// Configuration information for lkmv tool
/// This is the active configuration used by the application itself
/// When you want to load/save this configuration, it will become:
/// 1. [public_config::PublicConfig]: Configuration information that is saved to disk
/// 2. [secured_config::SecuredConfig]: Configuration information that is encrypted and saved to secure storage
#[derive(Debug)]
pub struct Config {
    /// Public readable config items when saved to disk
    pub public: PublicConfig,

    /// Private sensitive config items which are encrypted on disk
    pub private: ProtectedConfig,

    /// Root node of derivative keys
    pub bip32_root: ExtendedSigningKey,

    // Protected BIP32 seed
    pub bip32_seed: SecretString,

    /// Where did the key values come from? Derived or Imported?
    pub key_info: HashMap<String, KeyInfoConfig>,

    /// Persona DID and Document
    pub persona_did: PersonaDID,

    // *********************************************
    // Temporary Config values
    /// What protection method is being used for [SecuredConfig]
    pub protection_method: ProtectionMethod,

    /// Hardware token Admin PIN
    #[cfg(feature = "openpgp-card")]
    pub token_admin_pin: Option<SecretString>,

    /// Hardware token User PIN
    #[cfg(feature = "openpgp-card")]
    pub token_user_pin: SecretString,

    /// Unlock code if required
    pub unlock_code: Option<Vec<u8>>,

    /// Holds ATM profiles for relationships
    /// Key: Our local DID for the relationship
    /// NOTE: Does not hold the persona DID profile!
    pub atm_profiles: HashMap<Rc<String>, Arc<ATMProfile>>,

    /// All VRC's issued and received by VRC ID
    /// Key: VRC ID
    pub vrcs: HashMap<Rc<String>, Rc<DTGCredential>>,
}

/// Exported Configuration structure
#[derive(Deserialize, Serialize)]
pub struct ExportedConfig {
    pub pc: PublicConfig,
    pub sc: SecuredConfig,
}

/// Our public Persona DID used to identify ourselves within the Linux Foundation ecosystem
#[derive(Clone, Debug)]
pub struct PersonaDID {
    /// Resolved DID Document for this DID
    pub document: Document,

    /// Messaging Profile representing this DID within the TDK
    pub profile: Arc<ATMProfile>,
}

impl Config {
    /// Handles saving
    /// profile: Configuration profile name to use
    pub fn save(
        &self,
        profile: &str,
        #[cfg(feature = "openpgp-card")] touch_prompt: &(dyn Fn() + Send + Sync),
    ) -> Result<(), LKMVError> {
        self.public.save(
            profile,
            &self.private,
            &ProtectedConfig::get_seed(&self.bip32_root, "m/0'/0'/0'").unwrap(),
        )?;

        let sc = SecuredConfig::from(self);
        sc.save(
            profile,
            if let ConfigProtectionType::Token(token) = &self.public.protection {
                Some(token)
            } else {
                None
            },
            self.unlock_code.as_ref(),
            #[cfg(feature = "openpgp-card")]
            touch_prompt,
        )?;

        Ok(())
    }

    /// STEP 1 of loading the configuration,
    /// This can be used to determine if additional user information may be required to unlock the
    /// configuration.
    /// Specifically see the [PublicConfig::protection] as to what you may need to provide
    pub fn load_step1(profile: &str) -> Result<PublicConfig, LKMVError> {
        PublicConfig::load(profile)
    }

    /// STEP 2 of loading the configuration. Takes the output and additional information from
    /// [Config::load_step1]
    /// term: Console terminal manipulation
    /// tdk: Where secrets and config info will be stored
    /// profile: Configuration profile name to use
    /// unlock_passphrase: Optional if passed in from command line
    pub async fn load_step2(
        tdk: &mut TDK,
        profile: &str,
        public_config: PublicConfig,
        unlock_passphrase: Option<&UnlockCode>,
        #[cfg(feature = "openpgp-card")] token_user_pin: &SecretString,
        #[cfg(feature = "openpgp-card")] touch_prompt: &impl TokenInteractions,
    ) -> Result<Self, LKMVError> {
        let sc = SecuredConfig::load(
            profile,
            #[cfg(feature = "openpgp-card")]
            token_user_pin,
            if let ConfigProtectionType::Token(token) = &public_config.protection {
                Some(token)
            } else {
                None
            },
            unlock_passphrase,
            #[cfg(feature = "openpgp-card")]
            touch_prompt,
        )?;

        let bip32_root = ExtendedSigningKey::from_seed(
            BASE64_URL_SAFE_NO_PAD.decode(&sc.bip32_seed)?.as_slice(),
        )
        .map_err(|e| {
            LKMVError::BIP32(format!(
                "Couldn't get bip32 root from the secret seed material: {}",
                e
            ))
        })?;

        // Unencrypt the private config data
        let private_cfg = if let Some(private_cfg_str) = &public_config.private {
            ProtectedConfig::load(
                &ProtectedConfig::get_seed(&bip32_root, "m/0'/0'/0'")?,
                private_cfg_str,
            )?
        } else {
            ProtectedConfig::default()
        };

        // All config info has been loaded, load DID Document and regenerate keys
        let rr = tdk
            .did_resolver()
            .resolve(&public_config.persona_did)
            .await
            .map_err(|e| {
                LKMVError::Resolver(format!(
                    "Couldn't resolve Persona DID ({}): {}",
                    public_config.persona_did, e
                ))
            })?;

        // Create keys from DID Document
        Config::regenerate_persona_keys(tdk, &sc, &bip32_root, &rr.doc).await?;

        // Create persona profile
        let persona_profile = ATMProfile::new(
            tdk.atm.as_ref().unwrap(),
            Some("Persona DID".to_string()),
            public_config.persona_did.to_string(),
            Some(public_config.mediator_did.clone()),
        )
        .await?;

        // Add the persona profile to the TDK ATM Service
        // This allows it to send/receive messages directly to the Persona DID
        let atm = tdk.atm.clone().unwrap();
        let persona_profile = atm.profile_add(&persona_profile, true).await?;

        let atm_profiles = private_cfg
            .relationships
            .generate_profiles(
                tdk,
                &public_config.persona_did,
                &public_config.mediator_did,
                &bip32_root,
                &sc.key_info,
            )
            .await?;

        // Add all VRC's to the top level list
        let mut vrcs = HashMap::new();
        for relationship in private_cfg.vrcs_issued.values() {
            for (vrc_id, vrc) in relationship.iter() {
                vrcs.insert(vrc_id.clone(), vrc.clone());
            }
        }
        for relationship in private_cfg.vrcs_received.values() {
            for (vrc_id, vrc) in relationship.iter() {
                vrcs.insert(vrc_id.clone(), vrc.clone());
            }
        }

        Ok(Config {
            bip32_root,
            persona_did: PersonaDID {
                document: rr.doc,
                profile: persona_profile,
            },
            bip32_seed: SecretString::new(sc.bip32_seed.clone()),
            public: public_config,
            private: private_cfg,
            key_info: sc.key_info.clone(),
            #[cfg(feature = "openpgp-card")]
            token_admin_pin: None,
            #[cfg(feature = "openpgp-card")]
            token_user_pin: token_user_pin.clone(),
            protection_method: sc.protection_method.clone(),
            unlock_code: unlock_passphrase.map(|uc| uc.0.expose_secret().to_owned()),
            atm_profiles,
            vrcs,
        })
    }

    /// Returns the first matching set of keys for the persona DID
    /// This will pick the first:
    /// - Signing Key (assertion method)
    /// - Authentication (authentication)
    /// - Encryption (key agreement)
    ///
    pub async fn get_persona_keys(&self, tdk: &TDK) -> Result<PersonaDIDKeys, LKMVError> {
        let signing = if let Some(signing) = self.persona_did.document.assertion_method.first() {
            let Some(secret) = tdk
                .get_shared_state()
                .secrets_resolver
                .get_secret(signing.get_id())
                .await
            else {
                return Err(LKMVError::Config(format!(
                    "Couldn't find secret in TDK for ({})",
                    signing.get_id()
                )));
            };
            let Some(ki) = self.key_info.get(signing.get_id()) else {
                return Err(LKMVError::Config(format!(
                    "Couldn't find key info in lkmv Config for ({})",
                    signing.get_id()
                )));
            };
            KeyInfo {
                secret,
                source: ki.path.clone(),
                created: ki.create_time,
                expiry: None,
            }
        } else {
            return Err(LKMVError::Config(
                "DID Document does not contain any assertion methods!".to_string(),
            ));
        };

        let authentication =
            if let Some(authentication) = self.persona_did.document.authentication.first() {
                let Some(secret) = tdk
                    .get_shared_state()
                    .secrets_resolver
                    .get_secret(authentication.get_id())
                    .await
                else {
                    return Err(LKMVError::Config(format!(
                        "Couldn't find secret in TDK for ({})",
                        authentication.get_id()
                    )));
                };
                let Some(ki) = self.key_info.get(authentication.get_id()) else {
                    return Err(LKMVError::Config(format!(
                        "Couldn't find key info in lkmv Config for ({})",
                        authentication.get_id()
                    )));
                };
                KeyInfo {
                    secret,
                    source: ki.path.clone(),
                    created: ki.create_time,
                    expiry: None,
                }
            } else {
                return Err(LKMVError::Config(
                    "DID Document does not contain any authentication methods!".to_string(),
                ));
            };

        let decryption = if let Some(decryption) = self.persona_did.document.key_agreement.first() {
            let Some(secret) = tdk
                .get_shared_state()
                .secrets_resolver
                .get_secret(decryption.get_id())
                .await
            else {
                return Err(LKMVError::Config(format!(
                    "Couldn't find secret in TDK for ({})",
                    decryption.get_id()
                )));
            };
            let Some(ki) = self.key_info.get(decryption.get_id()) else {
                return Err(LKMVError::Config(format!(
                    "Couldn't find key info in lkmv Config for ({})",
                    decryption.get_id()
                )));
            };
            KeyInfo {
                secret,
                source: ki.path.clone(),
                created: ki.create_time,
                expiry: None,
            }
        } else {
            return Err(LKMVError::Config(
                "DID Document does not contain any key agreements!".to_string(),
            ));
        };
        Ok(PersonaDIDKeys {
            signing,
            authentication,
            decryption,
        })
    }

    /// Private function that regenerates the Persona DID keys from secured config
    async fn regenerate_persona_keys(
        tdk: &mut TDK,
        sc: &SecuredConfig,
        bip32_root: &ExtendedSigningKey,
        doc: &Document,
    ) -> Result<(), LKMVError> {
        // Rehydrate DID keys referenced by Verification Methods in the DID Document
        for vm in &doc.verification_method {
            let Some(kp) = sc.key_info.get(vm.id.as_str()) else {
                warn!(
                    "Couldn't find DID Verification method key path ({}) in config.",
                    vm.id
                );
                return Err(LKMVError::Config(format!(
                    "Couldn't find DID Verification method key path ({}) in config.",
                    vm.id
                )));
            };

            // need to match this to VM purpose
            let k_purpose = if doc.contains_key_agreement(vm.id.as_str()) {
                KeyPurpose::Encryption
            } else if doc.contains_authentication(vm.id.as_str()) {
                KeyPurpose::Authentication
            } else if doc.contains_assertion_method(vm.id.as_str()) {
                KeyPurpose::Signing
            } else {
                warn!("Unknown DID VM ({}) found", vm.id);
                continue;
            };

            let mut secret = match &kp.path {
                KeySourceMaterial::Derived { path } => {
                    bip32_root.get_secret_from_path(path, k_purpose)?
                }
                KeySourceMaterial::Imported { seed } => Secret::from_multibase(seed, None)
                    .map_err(|e| {
                        LKMVError::Secret(format!(
                            "Couldn't create secret from multibase for key id. Reason: {e}"
                        ))
                    })?,
            };

            // Set the Secret key ID correctly
            secret.id = vm.id.to_string();

            // Load the secret into the TDK Secrets resolver
            tdk.get_shared_state().secrets_resolver.insert(secret).await;
        }
        Ok(())
    }

    /// Exports the configuration settings to an encrypted file
    pub fn export(&self, passphrase: Option<SecretString>, file: &str) {
        let pc = PublicConfig::from(self);
        let sc = SecuredConfig::from(self);

        let seed_bytes = if let Some(passphrase) = passphrase {
            Sha256::digest(passphrase.expose_secret())
                .first_chunk::<32>()
                .expect("Couldn't get 32 bytes for passphrase hash")
                .to_owned()
        } else {
            Sha256::digest(
                Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter passphrase to encrypt exported configuration")
                    .with_confirmation("Confirm passphrase", "Passphrases do not match")
                    .interact()
                    .expect("Failed to read passphrase"),
            )
            .first_chunk::<32>()
            .expect("Couldn't get 32 bytes for passphrase hash")
            .to_owned()
        };

        let secured = match unlock_code_encrypt(
            &seed_bytes,
            &serde_json::to_vec(&ExportedConfig { pc, sc })
                .expect("Couldn't serialize Config settings"),
        ) {
            Ok(result) => result,
            Err(e) => {
                warn!("ERROR: Couldn't encrypt settings. Reason: {e}");
                return;
            }
        };

        match fs::write(file, BASE64_URL_SAFE_NO_PAD.encode(&secured)) {
            Ok(_) => {
                warn!("Successfully exported settings to file({file})");
            }
            Err(e) => {
                warn!("ERROR: Couldn't write to file ({file}). Reason: {e}");
            }
        }
    }

    /// Handles rejection of a VRC request
    pub fn handle_vrc_reject(
        &mut self,
        task_id: &Rc<String>,
        reason: Option<&str>,
        from: &Rc<String>,
    ) -> Result<(), LKMVError> {
        let reason = if let Some(reason) = reason {
            reason.to_string()
        } else {
            "NO REASON PROVIDED".to_string()
        };

        self.public.logs.insert(
            LogFamily::Relationship,
            format!(
                "Removed VRC ({}) request as rejected by remote entity Reason: {}",
                task_id, reason
            ),
        );

        self.private.tasks.remove(task_id);

        self.public.logs.insert(
            LogFamily::Task,
            format!(
                "VRC request rejected by remote DID({}) Task ID({}) Reason({})",
                from, task_id, reason
            ),
        );

        Ok(())
    }
}

// ****************************************************************************
// Key Types
// ****************************************************************************

/// Key Types used within lkmv
#[derive(Clone, Serialize, Default, Deserialize, Debug)]
pub enum KeyTypes {
    PersonaSigning,
    PersonaAuthentication,
    PersonaEncryption,
    PersonaOther,
    RelationshipVerification,
    RelationshipEncryption,
    WebVHManagement,
    #[default]
    Unknown,
}

impl Display for KeyTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            KeyTypes::PersonaSigning => "Persona Signing Key",
            KeyTypes::PersonaAuthentication => "Persona Authentication Key",
            KeyTypes::PersonaEncryption => "Persona Encryption Key",
            KeyTypes::PersonaOther => "Persona Other Key",
            KeyTypes::RelationshipVerification => "Relationship Verification Key",
            KeyTypes::RelationshipEncryption => "Relationship Encryption Key",
            KeyTypes::WebVHManagement => "Web VH Management Key",
            KeyTypes::Unknown => "Unknown Key Type",
        };
        write!(f, "{}", s)
    }
}

/// Secrets for the Persona DID
#[derive(Clone, Debug)]
pub struct PersonaDIDKeys {
    pub signing: KeyInfo,
    pub authentication: KeyInfo,
    pub decryption: KeyInfo,
}

/// Contains relevant key information required for setting up, configuring and managing keys
#[derive(Clone, Debug)]
pub struct KeyInfo {
    /// Secret Key Material that can be used within the TDK environment
    pub secret: Secret,
    /// Where did this key come from? Derived from BIP32 or Imported?
    pub source: KeySourceMaterial,

    /// Section 5.5.2 of RFC 4880 - Expiry time if set is # of days since creation
    pub expiry: Option<TimeDelta>,
    pub created: DateTime<Utc>,
}
