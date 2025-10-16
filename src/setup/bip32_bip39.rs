/*! BIP32 (derived keys) and BIP39 (mnemonic recovery phrases)
*  implementations live here
*/

use crate::{CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_RED};
use anyhow::{Context, Result, bail};
use bip39::Mnemonic;
use console::style;
use dialoguer::{Confirm, Input, theme::ColorfulTheme};
use ed25519_dalek_bip32::ExtendedSigningKey;
use rand::{RngCore, rng};
use zeroize::Zeroize;

// ****************************************************************************
// BIP32 Handling
// ****************************************************************************

/// Returns a BIP32 Master Key
pub fn get_bip32_root(seed: &[u8]) -> Result<ExtendedSigningKey> {
    ExtendedSigningKey::from_seed(seed).context("Couldn't create BIP32 Master Key from seed")
}

// ****************************************************************************
// BIP39 Mnemonic Handling
// ****************************************************************************

/// Prompts the user to enter their recovery phrase to recover entropy seed
pub fn mnemonic_from_recovery_phrase() -> Result<Mnemonic> {
    println!("{}", style("You can recover your secrets by entering your 24 word recovery phrase separated by whitespace below").color256(CLI_BLUE));

    fn inner() -> Result<Mnemonic> {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter your 24 word recovery phrase")
            .report(false)
            .interact_text()
            .context("Couldn't read recovery phrase from user input")?;

        // Check that the phrase looks valid
        let words: Vec<&str> = input.split_whitespace().collect();
        if words.len() != 24 {
            bail!("Recovery phrase must be 24 words long, got {}", words.len());
        }

        Mnemonic::parse_normalized(&input).context("Couldn't derive BIP39 mnemonic from words")
    }

    loop {
        match inner() {
            Ok(mnemonic) => {
                println!("{}", style("Recovery phrase accepted!").color256(CLI_GREEN));
                return Ok(mnemonic);
            }
            Err(e) => {
                println!("{}", style(e).color256(CLI_RED));

                if !Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Try again?")
                    .default(true)
                    .interact()
                    .unwrap()
                {
                    bail!("BIP39 Recovery failed")
                }
            }
        }
    }
}

/// Generates a new BIP39 Mnemonic that is used as a seed and recovery phrase
pub fn generate_bip39_mnemonic() -> Mnemonic {
    // Create 256 bits of entropy
    let mut entropy = [0u8; 32];
    let mut rng = rng();
    rng.fill_bytes(&mut entropy);

    match Mnemonic::from_entropy(&entropy) {
        Ok(mnemonic) => {
            entropy.zeroize(); // Clear entropy from memory

            println!(
                "\n{} {}",
                style("BIP39 Recovery Phrase").color256(CLI_BLUE),
                style("(Please store in a safe space):")
                    .color256(CLI_RED)
                    .blink()
            );
            println!(
                "{}",
                style(mnemonic.words().collect::<Vec<&str>>().join(" ")).color256(CLI_ORANGE)
            );
            println!();
            mnemonic
        }
        Err(e) => {
            panic!("Error creating BIP39 mnemonic from entropy: {e}");
        }
    }
}

// ****************************************************************************
// Tests
// ****************************************************************************

#[cfg(test)]
mod tests {
    use bip39::Mnemonic;

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
        let mnemonic = Mnemonic::parse_normalized(&words).unwrap();

        assert_eq!(mnemonic.to_entropy(), ENTROPY_BYTES);
    }
}
