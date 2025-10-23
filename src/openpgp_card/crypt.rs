/*! Encrypt/Decrypt functions using the openpgp-card
*/

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_RED,
    config::secured_config::{unlock_code_decrypt, unlock_code_encrypt},
    openpgp_card::open_card,
};
use anyhow::{Context, Result, bail};
use byteorder::{BigEndian, ByteOrder};
use console::{Term, style};
use openpgp_card::ocard::KeyType;
use openpgp_card_rpgp::CardSlot;
use pgp::{
    crypto::public_key::PublicKeyAlgorithm,
    ser::Serialize,
    types::{EskType, PkeskBytes},
};
use rand::Rng;
use secrecy::SecretString;
use std::io::BufReader;
use zeroize::Zeroize;

// Creates a simple 2-byte checksum over an array of bytes
fn generate_checksum(bytes: &[u8]) -> [u8; 2] {
    let sum = (bytes.iter().map(|v| u32::from(*v)).sum::<u32>() & 0xffff) as u16;

    let mut res = [0u8; 2];
    BigEndian::write_u16(&mut res[..], sum);

    res
}

/// Uses the decrypt public key on the token to encrypt a random Session Key (ESK)
/// Then encrypts the data with the session key using AES-GCM
///
/// Returns (ESK, encrypted data)
pub fn token_encrypt(token_id: &str, data: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut card =
        open_card(token_id).context(format!("Couldn't find hardware token ({})", token_id))?;
    let mut card = card
        .transaction()
        .context("Couldn't create hardware token transaction - encrypt")?;

    let cs = CardSlot::init_from_card(&mut card, KeyType::Decryption, &|| {
        eprintln!("Touch is required to get decrypt public-key")
    })?;

    // Create random 32 byte seed
    let mut seed: [u8; 32] = [0; 32];
    let mut rng = rand::thread_rng();
    rng.fill(&mut seed);

    // Augment the seed with Algo type (PlainText) and a 2-byte Checksum
    let mut seed_augmented: [u8; 35] = [0; 35];
    let cksum = generate_checksum(&seed);
    seed_augmented[1..33].copy_from_slice(&seed);
    seed_augmented[33] = cksum[0];
    seed_augmented[34] = cksum[1];

    // Get the public_key from the hardware token
    let pk = cs.public_key();
    let esk = pk.encrypt(rng, &seed_augmented, EskType::V6)?;

    // Encrypt the data payload using AES-GCM with the seed
    let encrypted = unlock_code_encrypt(&seed, data)?;

    // Get rid of raw secrets
    seed.zeroize();
    seed_augmented.zeroize();

    Ok((
        esk.to_bytes()
            .context("Couldn't convert encrypted ESK to bytes")?,
        encrypted,
    ))
}

/// Uses the decrypt key on the token to decrypt ESK
/// Then the secret seed from the ESK is used to decrypt the data payload using AES-GCM
pub fn token_decrypt(term: &Term, token_id: &str, esk: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    print!(
        "{}",
        style("Unlocking hardware token...").color256(CLI_BLUE)
    );
    term.hide_cursor()?;
    term.flush()?;

    let mut card =
        open_card(token_id).context(format!("Couldn't find hardware token ({})", token_id))?;
    let mut card = card
        .transaction()
        .context("Couldn't create hardware token transaction - decrypt")?;

    card.verify_user_pin(SecretString::new("123456".into()))?;
    card.to_user_card(None)?;

    term.show_cursor()?;
    println!(
        " {}",
        style("Success, token is in user mode").color256(CLI_GREEN)
    );
    print!(
        "{}",
        style("Decrypting SecuredConfig ESK from hardware token").color256(CLI_BLUE)
    );
    term.hide_cursor()?;
    term.flush()?;

    let cs = CardSlot::init_from_card(&mut card, KeyType::Decryption, &|| {
        eprintln!("Touch confirmation needed for decryption");
    })?;

    // Convert the raw ESK bytes back into a Public Key Encrypted Session Key
    let raw_br = BufReader::new(esk);
    let pk_esk = PkeskBytes::try_from_reader(&PublicKeyAlgorithm::ECDH, 6, raw_br)?;
    let (decrypted_esk, _) = cs.decrypt(&pk_esk)?;

    term.show_cursor()?;
    if decrypted_esk.len() != 32 {
        println!(
            " {}",
            style(format!(
                "Invalid ESK length ({}) received! Expected 32",
                decrypted_esk.len()
            ))
            .color256(CLI_RED)
        );
        bail!("Decrypted ESK has invalid length");
    }
    println!(
        " {}",
        style("Successfully recovered ESK for SecuredConfig").color256(CLI_GREEN)
    );

    // Can now decrypt the data payload using the ESK
    unlock_code_decrypt(decrypted_esk.first_chunk::<32>().unwrap(), data)
}
