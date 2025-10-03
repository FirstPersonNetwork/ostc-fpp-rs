/*! Handles the setup of the lkmv CLI tool
*/

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_RED,
    config::Config,
    setup::bip32_bip39::{generate_bip39_mnemonic, get_bip32_root, mnemonic_from_recovery_phrase},
};
use affinidi_tdk::secrets_resolver::secrets::Secret;
use anyhow::{Context, Result};
use bip39::Mnemonic;
use console::style;
use dialoguer::{Confirm, theme::ColorfulTheme};
use ed25519_dalek_bip32::DerivationPath;

mod bip32_bip39;

/// Sets up the CLI tool
pub fn cli_setup() -> Result<Config> {
    println!(
        "{}",
        style("Initial setup of the lkmv tool").color256(CLI_GREEN)
    );
    println!();

    // Are we recovering from a Recovery Phrase?
    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Recover Secrets from BIP39 recovery phrase?")
        .default(false)
        .interact()
        .unwrap()
    {
        // Using Recovery Phrase
        let mnemonic = mnemonic_from_recovery_phrase()?;
        create_keys(&mnemonic)?;
    } else if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Use (import) existing PGP Secrets?")
        .default(false)
        .interact()
        .unwrap()
    {
        // Import PGP Secret key material
        println!(
            "{}",
            style("Not implemented yet!").blink().color256(CLI_RED)
        );
    } else {
        // Creating new Secrets

        let mnemonic = generate_bip39_mnemonic();
        create_keys(&mnemonic)?;
    }

    Ok(Config {})
}

/// Creates the Secret Key Material required
fn create_keys(mnemonic: &Mnemonic) -> Result<()> {
    let bip32_master = get_bip32_root(mnemonic.to_entropy().as_slice())?;

    println!(
        "{}",
        style(
            "BIP32 Master Key sucessfully created. All necessary keys will be derived from this Key"
        )
        .color256(CLI_BLUE)
    );

    // Authentication key
    let auth_key = bip32_master
        .derive(&"m/0'/0'/0'".parse::<DerivationPath>().unwrap())
        .context("Failed to create ED25519 authentication key")?;
    let auth_secret = Secret::generate_ed25519(
        Some("authentication"),
        Some(auth_key.signing_key.as_bytes()),
    );
    println!(
        "{} {}",
        style("Authentication Key (ED25519) created:").color256(CLI_BLUE),
        style(auth_secret.get_public_keymultibase()?).color256(CLI_GREEN)
    );

    // Encryption key
    let enc_key = bip32_master
        .derive(&"m/0'/0'/1'".parse::<DerivationPath>().unwrap())
        .context("Failed to create X25519 encryption key")?;
    let enc_secret =
        Secret::generate_x25519(Some("encryption"), Some(enc_key.signing_key.as_bytes()))?;
    println!(
        "{} {}",
        style("Encryption Key (X25519) created:").color256(CLI_BLUE),
        style(enc_secret.get_public_keymultibase()?).color256(CLI_GREEN)
    );

    // Signing key
    let sign_key = bip32_master
        .derive(&"m/0'/0'/2'".parse::<DerivationPath>().unwrap())
        .context("Failed to create ED25519 encryption key")?;
    let sign_secret =
        Secret::generate_ed25519(Some("signing"), Some(sign_key.signing_key.as_bytes()));
    println!(
        "{} {}",
        style("Encryption Key (ED25519) created:").color256(CLI_BLUE),
        style(sign_secret.get_public_keymultibase()?).color256(CLI_GREEN)
    );
    Ok(())
}
