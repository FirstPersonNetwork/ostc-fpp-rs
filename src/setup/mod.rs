/*! Handles the setup of the lkmv CLI tool
*/

#[cfg(feature = "openpgp-card")]
use crate::setup::openpgp_card::setup_hardware_token;
use crate::{
    CLI_BLUE, CLI_GREEN,
    config::KeySourceMaterial,
    setup::{
        bip32_bip39::{generate_bip39_mnemonic, get_bip32_root, mnemonic_from_recovery_phrase},
        pgp_import::{PGPKeys, terminal_input_pgp_key},
    },
};
#[cfg(feature = "openpgp-card")]
use ::openpgp_card::ocard::KeyType;
use affinidi_tdk::secrets_resolver::secrets::Secret;
use anyhow::{Context, Result};
use bip39::Mnemonic;
use chrono::{DateTime, TimeDelta, Utc};
use console::style;
use dialoguer::{Confirm, theme::ColorfulTheme};
use ed25519_dalek_bip32::DerivationPath;
use std::fmt;

mod bip32_bip39;
#[cfg(feature = "openpgp-card")]
pub mod openpgp_card;
mod pgp_import;

/// Contains all setup information
#[derive(Default)]
pub struct SetupConfig {
    /// All secrets created during setup
    pub secrets: Vec<Secret>,

    /// Keyy path info (where did the keys come from?)
    pub key_paths: Vec<KeySourceMaterial>,
}

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
    /// Secret Key Material that can be used within the TDK Envirnonment
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
pub fn cli_setup() -> Result<()> {
    println!(
        "{}",
        style("Initial setup of the lkmv tool").color256(CLI_GREEN)
    );
    println!();

    // Are we recovering from a Recovery Phrase?
    let mnemonic = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Recover Secrets from BIP39 recovery phrase?")
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
        .with_prompt("Use (import) existing PGP Secrets?")
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
    setup_hardware_token(&c_did_keys)?;

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
            .context("Failed to create ED25519 signing key")?;
        let mut sign_secret =
            Secret::generate_ed25519(Some("sign"), Some(sign_key.signing_key.as_bytes()));

        sign_secret.id = sign_secret.get_public_keymultibase()?;

        println!(
            "{} {}",
            style("Signing Key (ED25519) created:").color256(CLI_BLUE),
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
            .context("Failed to create ED25519 authentication key")?;
        let mut auth_secret =
            Secret::generate_ed25519(Some("auth"), Some(auth_key.signing_key.as_bytes()));

        auth_secret.id = auth_secret.get_public_keymultibase()?;

        println!(
            "{} {}",
            style("Authentication Key (ED25519) created:").color256(CLI_BLUE),
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
