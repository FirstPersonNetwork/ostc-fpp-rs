/*! Handles the setup of the lkmv CLI tool
*/

use anyhow::{Context, Result};
use bip39::Mnemonic;
use console::style;
use dialoguer::{Confirm, theme::ColorfulTheme};
use rand::{RngCore, rng};
use zeroize::Zeroize;

use crate::{CLI_GREEN, CLI_RED, config::Config};

/// Sets up the CLI tool
pub fn cli_setup() -> Config {
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
        // Goto Recovery process
        println!(
            "{}",
            style("Not implemented yet!").blink().color256(CLI_RED)
        );
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

        let bip39 = generate_bip39();
        let words: Vec<&str> = bip39.words().collect();

        println!("Words\n{words:#?}")
    }

    Config {}
}

/// Generates a new BIP39 Mnemonic that is used as a seed and recovery phrase
fn generate_bip39() -> Mnemonic {
    // Create 256 bits of entropy
    let mut entropy = [0u8; 32];
    let mut rng = rng();
    rng.fill_bytes(&mut entropy);

    match Mnemonic::from_entropy(&entropy) {
        Ok(mnemonic) => {
            entropy.zeroize(); // Clear entropy from memory
            mnemonic
        }
        Err(e) => {
            panic!("Error creating BIP39 mnemonic from entropy: {e}");
        }
    }
}

/// Recovers seed entropy from a BIP39 Recovery Mnemonic phrase
/// words - The BIP39 Mnemonic phrase to recover from (whitespace separated words)
fn recover_bip39_from_mnemonic(words: &str) -> Result<Mnemonic> {
    Mnemonic::parse_normalized(words).context("Couldn't derive BIP39 mnemonic from words")
}

#[cfg(test)]
mod tests {
    use bip39::Mnemonic;

    use crate::setup::recover_bip39_from_mnemonic;

    const ENTROPY_BYTES: [u8; 32] = [
        7, 26, 142, 230, 65, 85, 188, 182, 29, 129, 52, 229, 217, 159, 243, 182, 73, 89, 196, 246,
        58, 28, 100, 144, 187, 21, 157, 39, 4, 188, 154, 180,
    ];

    const MNEMONIC_WORDS: [&str; 24] = [
        "alpha", "stamp", "ridge", "live", "forward", "force", "invite", "charge", "total",
        "smooth", "woman", "hold", "night", "tiny", "suggest", "drum", "goose", "magic", "shell",
        "demise", "icon", "furnace", "hello", "manual",
    ];

    #[test]
    fn test_generate_mnemonic() {
        let mnemonic =
            Mnemonic::from_entropy(&ENTROPY_BYTES).expect("Couldn't create mnemonic from entropy");

        for (index, word) in mnemonic.words().enumerate() {
            assert_eq!(MNEMONIC_WORDS[index], word);
        }
    }

    #[test]
    fn test_recover_mnemonic() {
        let words = MNEMONIC_WORDS.join(" ");
        let mnemonic = recover_bip39_from_mnemonic(&words)
            .expect("Couldn't create BIP39 mnemonic from recovery phrase!");

        assert_eq!(mnemonic.to_entropy(), ENTROPY_BYTES);
    }
}
