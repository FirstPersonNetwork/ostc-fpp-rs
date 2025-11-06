/*! Handles the setup of the lkmv CLI tool
*/

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_PURPLE, LF_PUBLIC_MEDIATOR_DID,
    config::{
        CommunityDID, Config,
        public_config::PublicConfig,
        secured_config::{KeyInfoConfig, KeySourceMaterial},
    },
    contacts::Contacts,
    setup::{
        bip32_bip39::{
            Bip32Extension, generate_bip39_mnemonic, get_bip32_root, mnemonic_from_recovery_phrase,
        },
        did::did_setup,
        pgp_export::ask_export_community_did_keys,
        pgp_import::{PGPKeys, terminal_input_pgp_key},
    },
};
#[cfg(feature = "openpgp-card")]
use crate::{
    openpgp_card::ui::{AdminPin, UserPin},
    setup::openpgp_card::setup_hardware_token,
};
#[cfg(feature = "openpgp-card")]
use ::openpgp_card::ocard::KeyType;
use affinidi_tdk::{
    TDK, common::config::TDKConfig, messaging::profiles::ATMProfile,
    secrets_resolver::secrets::Secret,
};
use anyhow::Result;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use bip39::Mnemonic;
use chrono::{DateTime, TimeDelta, Utc};
use console::{Term, style};
use dialoguer::{Confirm, Input, theme::ColorfulTheme};
use secrecy::SecretString;
use sha2::Digest;
use std::{collections::HashMap, fmt, sync::Arc};

pub mod bip32_bip39;
mod did;
#[cfg(feature = "openpgp-card")]
mod openpgp_card;
pub mod pgp_export;
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
#[derive(Debug)]
pub struct CommunityDIDKeys {
    pub signing: KeyInfo,
    pub authentication: KeyInfo,
    pub decryption: KeyInfo,
}

/// Sets up the CLI tool
pub async fn cli_setup(term: &Term) -> Result<()> {
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
    let mut c_did_keys = create_keys(&mnemonic, &imported_keys)?;

    // Export this as an armored PGP Keyfile?
    if imported_keys.is_empty() {
        ask_export_community_did_keys(term, &c_did_keys, None, None, true);
    }

    // Use hardware token?
    #[cfg(feature = "openpgp-card")]
    let token_id = {
        let mut admin_pin = AdminPin::default();
        setup_hardware_token(term, &mut admin_pin, &c_did_keys)?
    };
    #[cfg(not(feature = "openpgp-card"))]
    let token_id = None;

    // If hardware token is not being used, then ask for an unlock code
    let unlock_code = if token_id.is_none() {
        // Check if an unlock code is desired?
        create_unlock_code()
    } else {
        // No need for an unlock code when using hardware token
        None
    };

    // Use a different Mediator?
    let mediator_did = change_mediator();

    // Create a DID - will also rename the C-DID Keys with the right key-IDS
    let c_did = did_setup(
        get_bip32_root(mnemonic.to_entropy().as_slice())?,
        &mut c_did_keys,
        &mediator_did,
    )?;

    // Create Configuration
    let mut key_info = HashMap::new();
    key_info.insert(
        c_did_keys.signing.secret.id.clone(),
        KeyInfoConfig {
            path: c_did_keys.signing.source.clone(),
            create_time: c_did_keys.signing.created,
        },
    );
    key_info.insert(
        c_did_keys.authentication.secret.id.clone(),
        KeyInfoConfig {
            path: c_did_keys.authentication.source.clone(),
            create_time: c_did_keys.authentication.created,
        },
    );
    key_info.insert(
        c_did_keys.decryption.secret.id.clone(),
        KeyInfoConfig {
            path: c_did_keys.decryption.source.clone(),
            create_time: c_did_keys.decryption.created,
        },
    );

    // Instantiate TDK
    let tdk = TDK::new(
        TDKConfig::builder().with_load_environment(false).build()?,
        None,
    )
    .await?;

    // Initial Configuration state
    let config = Config {
        bip32_root: get_bip32_root(mnemonic.to_entropy().as_slice())?,
        bip32_seed: SecretString::new(BASE64_URL_SAFE_NO_PAD.encode(mnemonic.to_entropy())),
        public: PublicConfig {
            token_id,
            community_did: c_did.did.clone(),
            unlock_code: unlock_code.is_some(),
            mediator_did: mediator_did.clone(),
        },
        community_did: CommunityDID {
            document: c_did.document,
            profile: Arc::new(
                ATMProfile::new(
                    tdk.atm.as_ref().unwrap(),
                    Some("Community DID".to_string()),
                    c_did.did.clone(),
                    Some(mediator_did.clone()),
                )
                .await?,
            ),
        },
        key_info,
        #[cfg(feature = "openpgp-card")]
        token_admin_pin: AdminPin::default(),
        #[cfg(feature = "openpgp-card")]
        token_user_pin: UserPin::default(),
        contacts: Contacts::default(),
    };

    config.save(unlock_code.as_ref())?;

    Ok(())
}

/// Creates the Secret Key Material required
/// Returns the created Secrets and their source material
fn create_keys(mnemonic: &Mnemonic, imported_keys: &PGPKeys) -> Result<CommunityDIDKeys> {
    let bip32_root = get_bip32_root(mnemonic.to_entropy().as_slice())?;

    println!(
        "{}",
        style(
            "BIP32 Master Key successfully loaded. All necessary keys will be derived from this Key"
        )
        .color256(CLI_BLUE)
    );

    // Signing key
    let signing = if let Some(signing) = &imported_keys.signing {
        // use imported key
        signing.clone()
    } else {
        let mut sign_secret = bip32_root.get_secret_from_path("m/0'/0'/0'", KeyPurpose::Signing)?;

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
        let mut auth_secret =
            bip32_root.get_secret_from_path("m/0'/0'/1'", KeyPurpose::Authentication)?;

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
        let mut enc_secret =
            bip32_root.get_secret_from_path("m/0'/0'/2'", KeyPurpose::Encryption)?;

        enc_secret.id = enc_secret.get_public_keymultibase()?;

        println!(
            "TIMTAM: Public Hex: {}",
            hex::encode(enc_secret.get_public_bytes())
        );
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
        decryption: encryption,
    })
}

/// Generates a sha256 hash of an unlock code if required
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

/// Do you want to use an alternative mediator?
fn change_mediator() -> String {
    println!();
    println!("{}", style("lkmv utilizes DIDComm protocol to communicate. lkmv requires the use of a DIDComm Mediator to store and forward messages between parties privately and securely").color256(CLI_BLUE));
    println!(
        "{} {}",
        style("Default Mediator:").color256(CLI_BLUE),
        style(LF_PUBLIC_MEDIATOR_DID).color256(CLI_PURPLE),
    );

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to use an alternative DIDComm Mediator?")
        .default(false)
        .interact()
        .unwrap()
    {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("DIDComm Mediator DID:")
            .interact()
            .unwrap()
    } else {
        LF_PUBLIC_MEDIATOR_DID.to_string()
    }
}
