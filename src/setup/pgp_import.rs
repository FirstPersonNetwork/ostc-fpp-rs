/*! Handles the import of PGP Key Material
*   
*   Annoyingly PGP spec is convoluted and treats the primary key differently to the sub keys
*   So need to handle primary and subkeys separately
*/

use affinidi_tdk::secrets_resolver::secrets::Secret;
use anyhow::{Context, Result, bail};
use console::style;
use dialoguer::{Confirm, Editor, Password, theme::ColorfulTheme};
use pgp::{
    composed::{Deserializable, SignedSecretKey, SignedSecretSubKey},
    crypto::ecdh,
    packet::KeyFlags,
    types::{KeyDetails, PlainSecretParams, SecretParams},
};
use zeroize::Zeroize;

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
    /// Did we import any keys?
    pub fn is_empty(&self) -> bool {
        self.signing.is_none() && self.encryption.is_none() && self.authentication.is_none()
    }

    /// Confirms via the terminal if a valid imported key should be used for a specific purpose
    pub fn confirm_key_use(&mut self, flag: KeyFlags, secret: Secret) {
        if flag.sign()
            && self.signing.is_none()
            && Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Use this key for Signing?")
                .default(true)
                .interact()
                .unwrap_or(false)
        {
            self.signing = Some(secret.clone());
        }

        if (flag.encrypt_comms() || flag.encrypt_storage())
            && self.encryption.is_none()
            && Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Use this key for Encryption?")
                .default(true)
                .interact()
                .unwrap_or(false)
        {
            self.encryption = Some(secret.clone());
        }

        if flag.authentication()
            && self.authentication.is_none()
            && Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Use this key for Authentication?")
                .default(true)
                .interact()
                .unwrap_or(false)
        {
            self.authentication = Some(secret);
        }
    }

    pub fn import_sub_key(&mut self, key: &mut SignedSecretSubKey, password: &str) {
        println!("TIMTAM: {key:#?}");
        println!();

        println!(
            "{} {}",
            style("SubKey Fingerprint:").color256(CLI_BLUE),
            style(key.fingerprint()).color256(CLI_GREEN)
        );

        if unlock_pgp_sub_key(&mut key.key, password).is_err() {
            return;
        }

        let Some(signature) = key.signatures.first() else {
            println!(
                "{}",
                style("No key signature found for this subkey").color256(CLI_RED)
            );
            return;
        };

        show_key_purpose(signature.key_flags());

        let Ok(secret) = check_crypto_algo_type(key.secret_params(), signature.key_flags()) else {
            return;
        };
        self.confirm_key_use(signature.key_flags(), secret);
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

    println!();
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

    let imported = check_pgp_keys(&input)?;

    println!();
    println!("{}", style("PGP Imported Key Status:").color256(CLI_BLUE));
    if imported.is_empty() {
        println!(
            "  {}",
            style("No keys were imported from PGP!").color256(CLI_PURPLE)
        );
    } else {
        if let Some(key) = &imported.signing {
            println!(
                "  {} {}",
                style("Signing Public Key:").color256(CLI_BLUE),
                style(key.get_public_keymultibase()?).color256(CLI_GREEN)
            );
        }

        if let Some(key) = &imported.authentication {
            println!(
                "  {} {}",
                style("Authentication Public Key:").color256(CLI_BLUE),
                style(key.get_public_keymultibase()?).color256(CLI_GREEN)
            );
        }

        if let Some(key) = &imported.encryption {
            println!(
                "  {} {}",
                style("Encryption Public Key:").color256(CLI_BLUE),
                style(key.get_public_keymultibase()?).color256(CLI_GREEN)
            );
        }
    }
    Ok(imported)
}

/// Imports PGP Key structure from a export String
/// Returns a PGPKeys struct
pub fn check_pgp_keys(raw_key: &str) -> Result<PGPKeys> {
    let (mut keys, _) = SignedSecretKey::from_string(raw_key)?;

    // Try unlocking the key
    let mut password: String = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter PGP Key Passphrase (if no passphrase, leave blank)")
        .allow_empty_password(true)
        .interact()
        .unwrap_or_default();

    unlock_pgp_key(&mut keys.primary_key, &password)?;

    let mut imported = PGPKeys::default();

    // Process the PGP Primary Key and assign it to the right slot
    let (primary_flags, primary_secret) = extract_primary_key_details(&keys)?;
    imported.confirm_key_use(primary_flags, primary_secret);

    for k in keys.secret_subkeys.iter_mut() {
        imported.import_sub_key(k, &password);
    }

    password.zeroize();
    Ok(imported)
}

/// Extract important key info from the primary key
fn extract_primary_key_details(primary_key: &SignedSecretKey) -> Result<(KeyFlags, Secret)> {
    println!("TIMTAM: {primary_key:#?}");
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

    // Display the Key Purpose
    show_key_purpose(signature.key_flags());

    let secret = check_crypto_algo_type(
        primary_key.primary_key.secret_params(),
        signature.key_flags(),
    )?;

    Ok((signature.key_flags(), secret))
}

/// Prints the key purpose based on Key Flags
fn show_key_purpose(flags: KeyFlags) {
    // Key purpose from key_flags
    let mut flag = false;
    print!("{}", style("Key Purpose: ").color256(CLI_BLUE));
    if flags.sign() {
        print!("{}", style("Signing").color256(CLI_GREEN));
        flag = true;
    }

    if flags.encrypt_comms() || flags.encrypt_storage() {
        if flag {
            print!("{}", style(", ").color256(CLI_GREEN));
        }
        print!("{}", style("Encryption").color256(CLI_GREEN));
        flag = true;
    }

    if flags.authentication() {
        if flag {
            print!("{}", style(", ").color256(CLI_GREEN));
        }
        print!("{}", style("Authentication").color256(CLI_GREEN));
    }
    println!();
}

/// Ensures that only Curve25519 types are matched to the right purpose
fn check_crypto_algo_type(params: &SecretParams, flags: KeyFlags) -> Result<Secret> {
    let SecretParams::Plain(params) = params else {
        println!("{}", style("Expected to find encrypted secret parameters, instead received EncryptedSecretParams. Key was not unlocked properly").color256(CLI_RED));
        bail!("Key wasn't fully unlocked - ran into encrypted key secrets");
    };

    // Crypto algo check
    let mut secret = match params {
        PlainSecretParams::Ed25519(secret) | PlainSecretParams::Ed25519Legacy(secret) => {
            if flags.sign() || flags.authentication() {
                println!(
                    "{}",
                    style("Sucessfully retrieved Ed25519 Key Secret material").color256(CLI_GREEN)
                );
                Secret::generate_ed25519(None, Some(secret.as_bytes()))
            } else {
                println!(
                    "{}",
                    style("Ed25519 Key cannot be used for Encryption").color256(CLI_RED)
                );
                bail!("Invalid use of Ed25519 key");
            }
        }
        PlainSecretParams::X25519(secret) => {
            if flags.encrypt_comms() || flags.encrypt_storage() {
                // Valid use of X25519
                println!(
                    "{}",
                    style("Sucessfully retrieved X25519 Key Secret material").color256(CLI_GREEN)
                );
                Secret::generate_x25519(None, Some(secret.as_bytes()))?
            } else {
                println!(
                    "{}",
                    style("X25519 Key can only be used for Encryption").color256(CLI_RED)
                );
                bail!("Invalid use of X25519 key");
            }
        }
        PlainSecretParams::ECDH(secret) => {
            if (flags.encrypt_comms() || flags.encrypt_storage())
                && let ecdh::SecretKey::Curve25519(secret) = secret
            {
                // Valid use of X25519
                println!(
                    "{}",
                    style("Sucessfully retrieved X25519 Key Secret material").color256(CLI_GREEN)
                );
                Secret::generate_x25519(None, Some(secret.as_bytes()))?
            } else if let ecdh::SecretKey::Curve25519(_) = secret {
                println!(
                    "{}",
                    style("ECDH Key must be Curve25519!").color256(CLI_RED)
                );
                bail!("Invalid use of X25519 key");
            } else {
                println!(
                    "{}",
                    style("X25519 Key can only be used for Encryption").color256(CLI_RED)
                );
                bail!("Invalid use of X25519 key");
            }
        }
        _ => {
            println!(
                "{} {}",
                style("Invalid key Secret Parameters: ").color256(CLI_RED),
                style(format!("{:#?}", params)).color256(CLI_ORANGE)
            );
            bail!("Invalid key secret paramters");
        }
    };

    // Set the Key ID to be the base58 encoded public key (this can be used as a basic did:key:z...
    // DID)
    secret.id = secret.get_public_keymultibase()?;
    Ok(secret)
}

/// Unlocks the master PGP Key
fn unlock_pgp_key(key: &mut pgp::packet::SecretKey, password: &str) -> Result<()> {
    println!(
        "{}",
        style("Attempting to unlock and unencrypt Primary PGP key...").color256(CLI_BLUE)
    );

    key.remove_password(&pgp::types::Password::from(password.as_bytes()))
        .context("Couldn't remove Primary PGP Password")?;

    println!(
        "{}",
        style("Successfully unlocked Primary PGP key...").color256(CLI_GREEN)
    );
    println!();

    Ok(())
}

/// Unlocks the master PGP Key
fn unlock_pgp_sub_key(key: &mut pgp::packet::SecretSubkey, password: &str) -> Result<()> {
    println!(
        "{}",
        style("Attempting to unlock and unencrypt Sub PGP key...").color256(CLI_BLUE)
    );

    key.remove_password(&pgp::types::Password::from(password.as_bytes()))
        .context("Couldn't remove Sub PGP Password")?;

    println!(
        "{}",
        style("Successfully unlocked Sub PGP key...").color256(CLI_GREEN)
    );
    println!();

    Ok(())
}
