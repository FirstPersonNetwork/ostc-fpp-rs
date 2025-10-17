/*! Encrypt/Decrypt functions using the openpgp-card
*/

use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce, aead::Aead};
use anyhow::{Context, Result, bail};
use console::style;
use hkdf::Hkdf;
use openpgp_card::ocard::{KeyType, crypto::PublicKeyMaterial};
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey};

use crate::{CLI_ORANGE, CLI_RED, openpgp_card::open_card};

/// Uses the decrypt/encrypt key on the token to encrypt data
pub fn token_encrypt(token_id: &str, data: &[u8]) -> Result<Vec<u8>> {
    let mut card =
        open_card(token_id).context(format!("Couldn't find hardware token ({})", token_id))?;
    let mut card = card
        .transaction()
        .context("Couldn't create hardware token transaction - encrypt")?;

    // Get the public_key info for the decrypt key
    let public_key = if let PublicKeyMaterial::E(pk) = card
        .public_key_material(KeyType::Decryption)
        .context("Couldn't get public key from hardware token - encrypt")?
    {
        if let Some(bytes) = pk.data().first_chunk::<32>() {
            bytes.to_owned()
        } else {
            bail!("decrypt public key doesn't have 32 bytes!");
        }
    } else {
        bail!("Incorrect decrypt key type on hardware token. Must be ECC not RSA!");
    };

    let token_public_key = PublicKey::from(public_key);

    let mut rng = rand::thread_rng();
    //
    // Use X25519 to encrypt the data
    let ephemeral_secret = EphemeralSecret::random_from_rng(rng);

    let shared_secret = ephemeral_secret.diffie_hellman(&token_public_key);

    let hkdf = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
    let mut symmetric_key = [0u8; 32]; // For AES-256
    hkdf.expand(b"encryption key", &mut symmetric_key)
        .expect("Failed to expand HKDF");

    let key = Key::<Aes256Gcm>::from_slice(&symmetric_key);
    let cipher = Aes256Gcm::new(key);

    let nonce = Nonce::from_slice(b"lkmv nonce");
    match cipher.encrypt(nonce, data) {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            println!(
                "{}{}",
                style("ERROR: Couldn't encrypt data. Reason: ").color256(CLI_RED),
                style(e).color256(CLI_ORANGE)
            );
            bail!(e);
        }
    }
}
