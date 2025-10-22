/*! Encrypt/Decrypt functions using the openpgp-card
*/

use crate::openpgp_card::open_card;
use anyhow::{Context, Result, bail};
use openpgp_card::ocard::KeyType;
use openpgp_card_rpgp::CardSlot;
use pgp::{
    crypto::public_key::PublicKeyAlgorithm,
    ser::Serialize,
    types::{EskType, PkeskBytes},
};
use secrecy::SecretString;
use std::io::BufReader;

/// Uses the decrypt public key on the token to encrypt data
pub fn token_encrypt(token_id: &str, data: &[u8]) -> Result<Vec<u8>> {
    let mut card =
        open_card(token_id).context(format!("Couldn't find hardware token ({})", token_id))?;
    let mut card = card
        .transaction()
        .context("Couldn't create hardware token transaction - encrypt")?;

    let cs = CardSlot::init_from_card(&mut card, KeyType::Decryption, &|| {
        eprintln!("Touch is required to get decrypt public-key")
    })?;

    let rng = rand::thread_rng();
    let pk = cs.public_key();
    let encrypted = pk.encrypt(rng, data, EskType::V6)?;
    println!("TIMTAM: {:#?}", encrypted);

    println!("TIMTAM: {:?}", encrypted.to_bytes());
    Ok(encrypted.to_bytes()?)
}

/// Uses the decrypt key on the token to decrypt data
pub fn token_decrypt(token_id: &str, data: &[u8]) -> Result<Vec<u8>> {
    let mut card =
        open_card(token_id).context(format!("Couldn't find hardware token ({})", token_id))?;
    let mut card = card
        .transaction()
        .context("Couldn't create hardware token transaction - decrypt")?;

    card.verify_user_pin(SecretString::new("123456".into()))?;
    card.to_user_card(None)?;

    let cs = CardSlot::init_from_card(&mut card, KeyType::Decryption, &|| {
        eprintln!("Touch confirmation needed for decryption");
    })?;

    // Convert the raw bytes back into a Public Key Encrypted Session Key
    let raw_br = BufReader::new(data);
    let pk_esk = PkeskBytes::try_from_reader(&PublicKeyAlgorithm::ECDH, 6, raw_br)?;
    println!("TIMTAM: {:#?}", pk_esk);
    let (decrypted, key) = cs.decrypt(&pk_esk)?;

    println!("TIMTAM: Key_algo: {:#?}", key);

    Ok(decrypted)
}
