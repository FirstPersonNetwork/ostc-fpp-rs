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
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_RED,
    config::{
        public_config::PublicConfig,
        secured_config::{
            KeyInfoConfig, KeySourceMaterial, ProtectionMethod, SecuredConfig, unlock_code_encrypt,
        },
    },
    contacts::Contacts,
    get_unlock_code,
    setup::{CommunityDIDKeys, KeyInfo, KeyPurpose, bip32_bip39::Bip32Extension},
};
use affinidi_tdk::{
    TDK,
    did_common::{Document, document::DocumentExt},
    messaging::profiles::ATMProfile,
    secrets_resolver::{SecretsResolver, secrets::Secret},
};
use anyhow::{Context, Result, bail};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use console::{Term, style};
use dialoguer::{Password, theme::ColorfulTheme};
use ed25519_dalek_bip32::ExtendedSigningKey;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, sync::Arc};

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
    pub key_info: HashMap<String, KeyInfoConfig>,

    /// Community DID and Document
    pub community_did: CommunityDID,

    // *********************************************
    // Temporary Config values
    /// What protection method is being used for [SecuredConfig]
    pub protection_method: ProtectionMethod,

    #[cfg(feature = "openpgp-card")]
    /// Hardware token Admin PIN
    pub token_admin_pin: AdminPin,

    #[cfg(feature = "openpgp-card")]
    /// Hardware token User PIN
    pub token_user_pin: UserPin,

    /// Known contacts
    pub contacts: Contacts,

    /// Unlock code if required
    pub unlock_code: Option<[u8; 32]>,
}

/// Exported Configuration structure
#[derive(Deserialize, Serialize)]
pub struct ExportedConfig {
    pub pc: PublicConfig,
    pub sc: SecuredConfig,
}

/// Our public Community DID used to identify ourselves within the Linux Foundation ecosystem
#[derive(Debug)]
pub struct CommunityDID {
    /// Resolved DID Document for this DID
    pub document: Document,

    /// Messaging Profile representing this DID within the TDK
    pub profile: Arc<ATMProfile>,
}

impl Config {
    /// Handles saving
    pub fn save(&self) -> Result<()> {
        let pc = PublicConfig::from(self);
        pc.save()?;

        let sc = SecuredConfig::from(self);
        sc.save(self.public.token_id.as_ref(), self.unlock_code.as_ref())?;

        Ok(())
    }

    /// Loads Configuration from Public and Secured Configuration
    /// -term: Console terminal manipulation
    /// -tdk: Where secrets and config info will be stored
    /// unlock_code: Optional if passed in from command line
    pub async fn load(term: &Term, tdk: &mut TDK, unlock_code: Option<&str>) -> Result<Self> {
        let pc = PublicConfig::load().context("Couldn't load Public Configuration")?;

        let unlock_code = if let Some(unlock_code) = unlock_code {
            Some(sha2::Sha256::digest(unlock_code.as_bytes()).into())
        } else if pc.token_id.is_none() && pc.unlock_code {
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

        let bip32_root = ExtendedSigningKey::from_seed(
            BASE64_URL_SAFE_NO_PAD
                .decode(&sc.bip32_seed)
                .context("Couldn't base64 decode BIP32 seed")?
                .as_slice(),
        )?;
        // Create keys from DID Document
        Config::regenerate_community_keys(tdk, &sc, &bip32_root, &rr.doc).await?;

        let community_profile = ATMProfile::new(
            tdk.atm.as_ref().unwrap(),
            Some("Community DID".to_string()),
            pc.community_did.clone(),
            Some(pc.mediator_did.clone()),
        )
        .await?;

        // Add the community profile to the TDK ATM Service
        // This allows it to send/receive messages directly to the Community DID
        let atm = tdk.atm.clone().unwrap();
        let community_profile = atm.profile_add(&community_profile, true).await?;

        Ok(Config {
            bip32_root,
            community_did: CommunityDID {
                document: rr.doc,
                profile: community_profile,
            },
            bip32_seed: SecretString::new(sc.bip32_seed.clone()),
            public: pc,
            key_info: sc.key_info.clone(),
            #[cfg(feature = "openpgp-card")]
            token_admin_pin: AdminPin::default(),
            #[cfg(feature = "openpgp-card")]
            token_user_pin,
            contacts: sc.contacts.clone(),
            protection_method: sc.protection_method.clone(),
            unlock_code,
        })
    }

    /// Returns the first matching set of keys for the community DID
    /// This will pick the first:
    /// - Signing Key (assertion method)
    /// - Authentication (authentication)
    /// - Encryption (key agreement)
    ///
    pub async fn get_community_keys(&self, tdk: &TDK) -> Result<CommunityDIDKeys> {
        let signing = if let Some(signing) = self.community_did.document.assertion_method.first() {
            let Some(secret) = tdk
                .get_shared_state()
                .secrets_resolver
                .get_secret(signing.get_id())
                .await
            else {
                bail!("Couldn't find secret in TDK for ({})", signing.get_id());
            };
            let Some(ki) = self.key_info.get(signing.get_id()) else {
                bail!(
                    "Couldn't find key info in lkmv Config for ({})",
                    signing.get_id()
                );
            };
            KeyInfo {
                secret,
                source: ki.path.clone(),
                created: ki.create_time,
                expiry: None,
            }
        } else {
            bail!("DID Document does not contain any assertion methods!");
        };

        let authentication =
            if let Some(authentication) = self.community_did.document.authentication.first() {
                let Some(secret) = tdk
                    .get_shared_state()
                    .secrets_resolver
                    .get_secret(authentication.get_id())
                    .await
                else {
                    bail!(
                        "Couldn't find secret in TDK for ({})",
                        authentication.get_id()
                    );
                };
                let Some(ki) = self.key_info.get(authentication.get_id()) else {
                    bail!(
                        "Couldn't find key info in lkmv Config for ({})",
                        authentication.get_id()
                    );
                };
                KeyInfo {
                    secret,
                    source: ki.path.clone(),
                    created: ki.create_time,
                    expiry: None,
                }
            } else {
                bail!("DID Document does not contain any authentication methods!");
            };

        let decryption = if let Some(decryption) = self.community_did.document.key_agreement.first()
        {
            let Some(secret) = tdk
                .get_shared_state()
                .secrets_resolver
                .get_secret(decryption.get_id())
                .await
            else {
                bail!("Couldn't find secret in TDK for ({})", decryption.get_id());
            };
            let Some(ki) = self.key_info.get(decryption.get_id()) else {
                bail!(
                    "Couldn't find key info in lkmv Config for ({})",
                    decryption.get_id()
                );
            };
            KeyInfo {
                secret,
                source: ki.path.clone(),
                created: ki.create_time,
                expiry: None,
            }
        } else {
            bail!("DID Document does not contain any key agreements!");
        };
        Ok(CommunityDIDKeys {
            signing,
            authentication,
            decryption,
        })
    }

    /// Private function that regenerates the Community DID keys from secured config
    async fn regenerate_community_keys(
        tdk: &mut TDK,
        sc: &SecuredConfig,
        bip32_root: &ExtendedSigningKey,
        doc: &Document,
    ) -> Result<()> {
        // Rehydrate DID keys referenced by Verification Methods in the DID Document
        for vm in &doc.verification_method {
            let Some(kp) = sc.key_info.get(vm.id.as_str()) else {
                bail!(
                    "Couldn't find DID Verification method key path ({}) in config.",
                    vm.id
                );
            };

            // need to match this to VM purpose
            let k_purpose = if doc.contains_key_agreement(vm.id.as_str()) {
                KeyPurpose::Encryption
            } else if doc.contains_authentication(vm.id.as_str()) {
                KeyPurpose::Authentication
            } else if doc.contains_assertion_method(vm.id.as_str()) {
                KeyPurpose::Signing
            } else {
                println!(
                    "{}",
                    style("WARN: Unknown DID VM found").color256(CLI_ORANGE)
                );
                continue;
            };

            let mut secret = match &kp.path {
                KeySourceMaterial::Derived { path } => {
                    bip32_root.get_secret_from_path(path, k_purpose)?
                }
                KeySourceMaterial::Imported { seed } => Secret::from_multibase(seed, None)?,
            };

            // Set the Secret key ID correctly
            secret.id = vm.id.to_string();

            // Load the secret into the TDK Secrets resolver
            tdk.get_shared_state().secrets_resolver.insert(secret).await;
        }
        Ok(())
    }

    /// Prints information relating to the configuration to console
    pub fn status(&self) {
        println!("{}", style("Configured Keys:").color256(CLI_BLUE));
        for (k, v) in &self.key_info {
            println!(
                "  {} {} {} {}",
                style("Key #id:").color256(CLI_BLUE),
                style(k).color256(CLI_GREEN),
                style("Created:").color256(CLI_BLUE),
                style(v.create_time).color256(CLI_GREEN)
            );
        }
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
                println!(
                    "{}{}",
                    style("ERROR: Couldn't encrypt settings. Reason: ").color256(CLI_RED),
                    style(e).color256(CLI_ORANGE)
                );
                return;
            }
        };

        match fs::write(file, BASE64_URL_SAFE_NO_PAD.encode(&secured)) {
            Ok(_) => {
                println!(
                    "{}{}{}",
                    style("Successfully exported settings to file(").color256(CLI_GREEN),
                    style(file).color256(CLI_PURPLE),
                    style(")").color256(CLI_GREEN)
                );
            }
            Err(e) => {
                println!(
                    "{}{}{}{}",
                    style("ERROR: Couldn't write to file (").color256(CLI_RED),
                    style(file).color256(CLI_PURPLE),
                    style(". Reason: ").color256(CLI_RED),
                    style(e).color256(CLI_ORANGE)
                );
            }
        }
    }
}
