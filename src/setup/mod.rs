/*! Handles the setup of the lkmv CLI tool
*/

#[cfg(feature = "openpgp-card")]
use crate::setup::openpgp_card::setup_hardware_token;
use crate::{
    CLI_BLUE, CLI_GREEN,
    config::{CommunityDID, Config, KeySourceMaterial, secured_config::SecuredConfig},
    setup::{
        bip32_bip39::{generate_bip39_mnemonic, get_bip32_root, mnemonic_from_recovery_phrase},
        pgp_import::{PGPKeys, terminal_input_pgp_key},
    },
};
#[cfg(feature = "openpgp-card")]
use ::openpgp_card::ocard::KeyType;
use affinidi_tdk::{did_common::Document, secrets_resolver::secrets::Secret};
use anyhow::{Context, Result};
use bip39::Mnemonic;
use chrono::{DateTime, TimeDelta, Utc};
use console::{Term, style};
use dialoguer::{Confirm, theme::ColorfulTheme};
use ed25519_dalek_bip32::DerivationPath;
use sha2::Digest;
use std::{collections::HashMap, fmt};

mod bip32_bip39;
#[cfg(feature = "openpgp-card")]
pub mod openpgp_card;
mod pgp_import;

/// Tags what the key is used for
#[derive(Default, Debug, PartialEq)]
pub enum KeyPurpose {
    Signing,
    Authentication,
    Encryption,
    #[default]
    Unknown,
}

impl fmt::Display for KeyPurpose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyPurpose::Signing => write!(f, "Signing"),
            KeyPurpose::Authentication => write!(f, "Authentication"),
            KeyPurpose::Encryption => write!(f, "Encryption"),
            KeyPurpose::Unknown => write!(f, "Unknown"),
        }
    }
}

#[cfg(feature = "openpgp-card")]
impl From<KeyType> for KeyPurpose {
    fn from(kt: KeyType) -> Self {
        match kt {
            KeyType::Signing => KeyPurpose::Signing,
            KeyType::Authentication => KeyPurpose::Authentication,
            KeyType::Decryption => KeyPurpose::Encryption,
            _ => KeyPurpose::Unknown,
        }
    }
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

/// Secrets for the Community DID
pub struct CommunityDIDKeys {
    pub signing: KeyInfo,
    pub authentication: KeyInfo,
    pub encryption: KeyInfo,
}

/// Sets up the CLI tool
pub fn cli_setup(term: &Term) -> Result<()> {
    println!(
        "{}",
        style("Initial setup of the lkmv tool").color256(CLI_GREEN)
    );
    println!();

    // Are we recovering from a Recovery Phrase?
    let mnemonic = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Recover Secrets from 24 word recovery phrase?")
        .default(false)
        .interact()
        .unwrap()
    {
        // Using Recovery Phrase
        mnemonic_from_recovery_phrase()?
    } else {
        generate_bip39_mnemonic()
    };

    let imported_keys = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Use (import) existing PGP keys?")
        .default(false)
        .interact()
        .unwrap()
    {
        // Import PGP Secret key material
        terminal_input_pgp_key()?
    } else {
        PGPKeys::default()
    };

    // Creating new Secrets for the Community DID
    let c_did_keys = create_keys(&mnemonic, &imported_keys)?;

    // Use hardware token?
    #[cfg(feature = "openpgp-card")]
    let token_id = setup_hardware_token(term, &c_did_keys)?;
    #[cfg(not(feature = "openpgp-card"))]
    let token_id = None;

    // Create Configuration
    let mut key_path = HashMap::new();
    key_path.insert(
        c_did_keys.signing.secret.id.clone(),
        c_did_keys.signing.source.clone(),
    );
    key_path.insert(
        c_did_keys.authentication.secret.id.clone(),
        c_did_keys.authentication.source.clone(),
    );
    key_path.insert(
        c_did_keys.encryption.secret.id.clone(),
        c_did_keys.encryption.source.clone(),
    );

    // If hardware token is not being used, then ask for an unlock code
    let unlock_code = if token_id.is_none() {
        // Check if an unlock code is desired?
        create_unlock_code()
    } else {
        // No need for an unlock code when using hardware token
        None
    };

    // Try saving the bip32 seed to OS Secure Store
    let sc = SecuredConfig::new(mnemonic.to_entropy().as_slice());
    sc.initial_save(token_id.as_ref(), unlock_code.as_ref())?;

    let config = Config {
        bip32_seed: get_bip32_root(mnemonic.to_entropy().as_slice())?,
        token_id,
        keys_path: key_path,
        // TODO: Replace this with correct DID
        community_did: CommunityDID {
            id: "TODO".to_string(),
            document: Document::new("did:todo:fix_this_later")?,
        },
        unlock_code: unlock_code.is_some(),
    };

    config.save()?;

    Ok(())
}

/// Creates the Secret Key Material required
/// Returns the created Secrets and their source material
fn create_keys(mnemonic: &Mnemonic, imported_keys: &PGPKeys) -> Result<CommunityDIDKeys> {
    let bip32_master = get_bip32_root(mnemonic.to_entropy().as_slice())?;

    println!(
        "{}",
        style(
            "BIP32 Master Key sucessfully loaded. All necessary keys will be derived from this Key"
        )
        .color256(CLI_BLUE)
    );

    // Signing key
    let signing = if let Some(signing) = &imported_keys.signing {
        // use imported key
        signing.clone()
    } else {
        let sign_key = bip32_master
            .derive(&"m/0'/0'/0'".parse::<DerivationPath>().unwrap())
            .context("Failed to create Ed25519 signing key")?;
        let mut sign_secret =
            Secret::generate_ed25519(Some("sign"), Some(sign_key.signing_key.as_bytes()));

        sign_secret.id = sign_secret.get_public_keymultibase()?;

        println!(
            "{} {}",
            style("Signing Key (Ed25519) created:").color256(CLI_BLUE),
            style(&sign_secret.id).color256(CLI_GREEN)
        );

        KeyInfo {
            secret: sign_secret,
            source: KeySourceMaterial::Derived {
                path: "m/0'/0'/0'".to_string(),
            },
            expiry: None,
            created: Utc::now(),
        }
    };

    // Authentication key
    let authentication = if let Some(authentication) = &imported_keys.authentication {
        // use imported key
        authentication.clone()
    } else {
        let auth_key = bip32_master
            .derive(&"m/0'/0'/1'".parse::<DerivationPath>().unwrap())
            .context("Failed to create Ed25519 authentication key")?;
        let mut auth_secret =
            Secret::generate_ed25519(Some("auth"), Some(auth_key.signing_key.as_bytes()));

        auth_secret.id = auth_secret.get_public_keymultibase()?;

        println!(
            "{} {}",
            style("Authentication Key (Ed25519) created:").color256(CLI_BLUE),
            style(&auth_secret.id).color256(CLI_GREEN)
        );

        KeyInfo {
            secret: auth_secret,
            source: KeySourceMaterial::Derived {
                path: "m/0'/0'/1'".to_string(),
            },
            expiry: None,
            created: Utc::now(),
        }
    };

    // Encryption key
    let encryption = if let Some(encryption) = &imported_keys.encryption {
        // use imported key
        encryption.clone()
    } else {
        let enc_key = bip32_master
            .derive(&"m/0'/0'/2'".parse::<DerivationPath>().unwrap())
            .context("Failed to create X25519 encryption key")?;
        let mut enc_secret =
            Secret::generate_ed25519(Some("enc"), Some(enc_key.signing_key.as_bytes()));

        enc_secret.id = enc_secret.get_public_keymultibase()?;

        println!(
            "{} {}",
            style("Encryption Key (X25519) created:").color256(CLI_BLUE),
            style(&enc_secret.id).color256(CLI_GREEN)
        );
        KeyInfo {
            secret: enc_secret,
            source: KeySourceMaterial::Derived {
                path: "m/0'/0'/2'".to_string(),
            },
            expiry: None,
            created: Utc::now(),
        }
    };

    Ok(CommunityDIDKeys {
        signing,
        authentication,
        encryption,
    })
}

fn create_unlock_code() -> Option<[u8; 32]> {
    println!("{}", style("NOTE: You are not using any hardware token. While secret information will be stored in your OS secure store where possible, it is best practice to protect this data with an unlock code.").color256(CLI_BLUE));
    println!("  {}", style("This unlock code is asked on application start so it can unlock secret configuration data required.").color256(CLI_BLUE));

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to set an unlock code to protect your secrets?")
        .default(true)
        .interact()
        .unwrap()
    {
        // Get unlock code from terminal
        let unlock_code: String = dialoguer::Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter Unlock Code")
            .with_confirmation("Confirm Unlock Code", "Unlock Codes do not match")
            .interact()
            .unwrap();

        // Create SHA2-256 hash of the unlock code
        Some(sha2::Sha256::digest(unlock_code.as_bytes()).into())
    } else {
        None
    }
}
