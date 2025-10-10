/*! Handles the import of PGP Key Material
*   
*   Annoyingly PGP spec is convoluted and treats the primary key differently to the sub keys
*   So need to handle primary and sub-keys separately
*/

use affinidi_tdk::secrets_resolver::secrets::Secret;
use anyhow::{Context, Result, bail};
use console::style;
use dialoguer::{Confirm, Editor, Password, theme::ColorfulTheme};
use pgp::{
    composed::{Deserializable, SignedSecretKey, SignedSecretSubKey},
    crypto::public_key::PublicKeyAlgorithm,
    packet::KeyFlags,
    types::{KeyDetails, PlainSecretParams, SecretParams},
};

use crate::{CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_RED};

/// Holds imported PGP Keys
#[derive(Default)]
pub struct PGPKeys {
    /// PGP Signing Key (Must be Ed25519)
    pub signing: Option<Secret>,

    /// PGP Encryption Key (Must be X25519)
    pub encryption: Option<Secret>,

    /// PGP Authentication Key (Must be Ed25519)
    pub authentication: Option<Secret>,
}

impl PGPKeys {
    pub fn import_sub_key(&mut self, key: &SignedSecretSubKey) {
        println!("\n************************************************************");
        println!("Algo: {:#?}", key.algorithm());
        println!("Key ID: {:#?}", key.key_id());
        println!("Fingerprint: {:#?}", key.fingerprint());

        for sigs in &key.signatures {
            println!("\tflags: sign: {}", sigs.key_flags().sign());
            println!("\tflags: encrypt: {}", sigs.key_flags().encrypt_comms());
            println!("\tflags: auth: {}", sigs.key_flags().authentication());
        }
    }
}

/// Handles terminal input of a PGP Key
pub fn terminal_input_pgp_key() -> Result<PGPKeys> {
    println!(
        "{}",
        style("You are going to be prompted to enter pre-created PGP Private key details.")
            .color256(CLI_BLUE)
    );
    println!();
    println!(
        "{}",
        style("The key format must look like the following:").color256(CLI_BLUE)
    );
    println!(
        "\t{}",
        style("-----BEGIN PGP PRIVATE KEY BLOCK-----").color256(CLI_PURPLE)
    );
    println!(
        "\n\t{}",
        style("<PRIVATE KEY MATERIAL>").color256(CLI_PURPLE)
    );
    println!(
        "\t{}\n",
        style("-----END PGP PRIVATE KEY BLOCK-----").color256(CLI_PURPLE)
    );
    println!(
        "{}",
        style("This PGP Private Key must be the export of a PGP Key with the following details:")
            .color256(CLI_BLUE)
    );
    println!(
        "\t{}",
        style("1. Must contain at most 3 keys (1 Primary, 0 to 2 Sub Keys").color256(CLI_BLUE)
    );
    println!(
        "\t{}",
        style("2. Signing and Authentication keys must be ED25519").color256(CLI_BLUE)
    );
    println!(
        "\t{}",
        style("3. Encryption key must be X25519").color256(CLI_BLUE)
    );
    println!();
    println!(
        "\t{}",
        style("NOTE: If a key is invalid for any reason, it will be ignored").color256(CLI_ORANGE)
    );
    println!(
        "\t{}",
        style("NOTE: Key Expiry will be honored, key rotation is up to the user to manage")
            .color256(CLI_ORANGE)
    );
    println!();
    println!(
        "\t{}",
        style("Any missing key information will be auto-generated from the BIP32 root")
            .color256(CLI_ORANGE)
    );

    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Continue?")
        .default(true)
        .interact()
        .unwrap_or(false)
    {
        bail!("PGP Import aborted by user")
    }

    let input: String = match Editor::new()
        .edit("Paste your Private PGP Key here")
        .context("An error occurred importing PGP Private Key")?
    {
        Some(input) => input,
        _ => {
            bail!("Aborted PGP Key Import");
        }
    };

    check_pgp_keys(&input)
}

/// Imports PGP Key structure from a export String
/// Returns a PGPKeys struct
pub fn check_pgp_keys(raw_key: &str) -> Result<PGPKeys> {
    let (mut keys, _) = SignedSecretKey::from_string(raw_key)?;

    // Try unlocking the key
    let password: String = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter PGP Key Passphrase (if no passphrase, leave blank)")
        .allow_empty_password(true)
        .interact()
        .unwrap_or_default();

    println!(
        "{}",
        style("Attempting to unlock and unencrypt PGP keys...").color256(CLI_BLUE)
    );

    keys.primary_key
        .remove_password(&pgp::types::Password::from(password.as_bytes()))
        .context("Couldn't remove PGP Password")?;

    println!(
        "{}",
        style("Successfully unlocked PGP keys...").color256(CLI_GREEN)
    );
    println!();

    let mut imported = PGPKeys::default();

    extract_primary_key_details(&keys)?;

    println!(
        "\n{}",
        style("Reading in sub-keys is a work in progress...")
            .color256(CLI_ORANGE)
            .blink()
    );
    for k in keys.secret_subkeys {
        imported.import_sub_key(&k);
    }

    Ok(imported)
}

/// Extract important key info from the primary key
fn extract_primary_key_details(primary_key: &SignedSecretKey) -> Result<(KeyFlags, Secret)> {
    let Some(user) = primary_key.details.users.first() else {
        println!(
            "{}",
            style("Couldn't find a valid user in the PGP Primary key!").color256(CLI_RED)
        );
        bail!("Invalid User in the PGP Primary key!");
    };

    println!(
        "{} {}",
        style("Primary Key Fingerprint:").color256(CLI_BLUE),
        style(primary_key.primary_key.fingerprint()).color256(CLI_GREEN)
    );

    print!("{} ", style("Primary Key User:").color256(CLI_BLUE));
    if let Some(user) = user.id.as_str() {
        println!("{}", style(user).color256(CLI_GREEN));
    } else {
        println!("{}", style("UNKNOWN").color256(CLI_ORANGE));
    }

    let Some(signature) = user.signatures.first() else {
        println!(
            "{}",
            style("No key signature found for the primary key").color256(CLI_RED)
        );
        bail!("No key signature found for the primary key!");
    };

    // Key purpose from key_flags
    let mut flag = false;
    print!("{}", style("Primary Key Purpose: ").color256(CLI_BLUE));
    if signature.key_flags().sign() {
        print!("{}", style("Signing").color256(CLI_GREEN));
        flag = true;
    }

    if signature.key_flags().encrypt_comms() || signature.key_flags().encrypt_storage() {
        if flag {
            print!("{}", style(", ").color256(CLI_GREEN));
        }
        print!("{}", style("Encryption").color256(CLI_GREEN));
        flag = true;
    }

    if signature.key_flags().authentication() {
        if flag {
            print!("{}", style(", ").color256(CLI_GREEN));
        }
        print!("{}", style("Authentication").color256(CLI_GREEN));
    }
    println!();

    // Crypto algo check
    match primary_key.primary_key.algorithm() {
        PublicKeyAlgorithm::EdDSALegacy => {
            if !signature.key_flags().sign() && !signature.key_flags().authentication() {
                println!(
                    "{}{}",
                    style("Invalid key crypto algorithm. Expected Ed25519 variant. Receieved: ")
                        .color256(CLI_RED),
                    style(format!("{:?}", primary_key.primary_key.algorithm()))
                        .on_color256(CLI_ORANGE)
                );
                bail!("Invalid key crypto algorithm");
            }
            println!(
                "{} {}",
                style("Primary Key Algo:").color256(CLI_BLUE),
                style("Ed25519 (Legacy)").color256(CLI_GREEN)
            )
        }
        _ => {
            println!(
                "{}{}",
                style("Invalid key crypto algorithm. No Curve25519 based algo found. Receieved: ")
                    .color256(CLI_RED),
                style(format!("{:?}", primary_key.primary_key.algorithm())).on_color256(CLI_ORANGE)
            );
            bail!("Invalid key crypto algorithm");
        }
    }

    let secret = if let SecretParams::Plain(params) = primary_key.primary_key.secret_params() {
        match params {
            PlainSecretParams::Ed25519(secret) | PlainSecretParams::Ed25519Legacy(secret) => {
                println!(
                    "{}",
                    style("Sucessfully retrieved Ed25519 Primary Key Secret material")
                        .color256(CLI_GREEN)
                );
                Secret::generate_ed25519(None, Some(secret.as_bytes()))
            }
            PlainSecretParams::X25519(secret) => {
                println!(
                    "{}",
                    style("Sucessfully retrieved X25519 Primary Key Secret material")
                        .color256(CLI_GREEN)
                );
                Secret::generate_x25519(None, Some(secret.as_bytes()))?
            }
            _ => {
                println!(
                    "{}",
                    style("Invalid primary key Secret Parameters").color256(CLI_RED)
                );
                bail!("Invalid primary key secret paramters");
            }
        }
    } else {
        println!("{}", style("Expected to find encrypted secret parameters, instead received EncryptedSecretParams. Key was not unlocked properly").color256(CLI_RED));
        bail!("Key wasn't fully unlocked - ran into encrypted key secrets");
    };

    Ok((signature.key_flags(), secret))
}
