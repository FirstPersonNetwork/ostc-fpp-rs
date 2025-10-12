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
use affinidi_tdk::secrets_resolver::secrets::Secret;
use anyhow::{Context, Result};
use bip39::Mnemonic;
use console::style;
use dialoguer::{Confirm, theme::ColorfulTheme};
use ed25519_dalek_bip32::DerivationPath;

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

/// Secrets for the Community DID
pub struct CommunityDIDKeys {
    pub signing: Secret,
    pub signing_path: KeySourceMaterial,
    pub authentication: Secret,
    pub authentication_path: KeySourceMaterial,
    pub encryption: Secret,
    pub encryption_path: KeySourceMaterial,
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
    let (signing, signing_path) = if let Some(signing) = imported_keys.signing.clone() {
        // use imported key
        (
            signing.clone(),
            KeySourceMaterial::Imported {
                key_id: signing.id.clone(),
            },
        )
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
        (
            sign_secret,
            KeySourceMaterial::Derived {
                path: "m/0'/0'/0'".to_string(),
            },
        )
    };

    // Authentication key
    let (authentication, authentication_path) =
        if let Some(authentication) = imported_keys.authentication.clone() {
            // use imported key
            (
                authentication.clone(),
                KeySourceMaterial::Imported {
                    key_id: authentication.id.clone(),
                },
            )
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
            (
                auth_secret,
                KeySourceMaterial::Derived {
                    path: "m/0'/0'/1'".to_string(),
                },
            )
        };

    // Encryption key
    let (encryption, encryption_path) = if let Some(encryption) = imported_keys.encryption.clone() {
        // use imported key
        (
            encryption.clone(),
            KeySourceMaterial::Imported {
                key_id: encryption.id.clone(),
            },
        )
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
        (
            enc_secret,
            KeySourceMaterial::Derived {
                path: "m/0'/0'/2'".to_string(),
            },
        )
    };

    Ok(CommunityDIDKeys {
        signing,
        signing_path,
        authentication,
        authentication_path,
        encryption,
        encryption_path,
    })
}
