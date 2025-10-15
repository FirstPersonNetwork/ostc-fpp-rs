/*!
*   Handles writing of data to the OpenPGP Card
*/

use crate::{
    CLI_BLUE, CLI_GREEN,
    setup::{CommunityDIDKeys, KeyInfo, KeyPurpose},
};
use anyhow::{Result, bail};
use chrono::Utc;
use console::style;
use dialoguer::{Password, theme::ColorfulTheme};
use ed25519_dalek_bip32::VerifyingKey;
use openpgp_card::{Card, ocard::KeyType, state::Open};
use openpgp_card_rpgp::UploadableKey;
use pgp::{
    crypto::{self, ed25519::Mode, public_key::PublicKeyAlgorithm},
    packet::{PacketHeader, PublicKey, SecretKey},
    types::{
        EcdhPublicParams, EddsaLegacyPublicParams, KeyVersion, PlainSecretParams, PublicParams,
        SecretParams, Tag,
    },
};
use secrecy::SecretString;
use x25519_dalek::StaticSecret;

/// Writes keys to the card
pub fn write_keys_to_card(card: &mut Card<Open>, keys: &CommunityDIDKeys) -> Result<()> {
    // Open the card in admin mode
    let admin_pin: SecretString = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Admin PIN")
        .allow_empty_password(true)
        .interact()
        .unwrap()
        .into();

    // Try unlocking the card with the admin PIN
    let mut open_card = card.transaction()?;
    open_card.verify_admin_pin(admin_pin)?;
    let mut card = open_card.to_admin_card(None)?;

    // Create a PGP secret key packet
    println!("{}", style("Writing Signing key...").color256(CLI_BLUE));
    let uk = create_pgp_secret_packet(&keys.signing, KeyPurpose::Signing)?;
    card.import_key(Box::new(uk), KeyType::Signing)?;
    println!("  {}", style("Success").color256(CLI_GREEN));

    println!(
        "{}",
        style("Writing Authentication key...").color256(CLI_BLUE)
    );
    let uk = create_pgp_secret_packet(&keys.authentication, KeyPurpose::Authentication)?;
    card.import_key(Box::new(uk), KeyType::Authentication)?;
    println!("  {}", style("Success").color256(CLI_GREEN));

    println!("{}", style("Writing Encryption key...").color256(CLI_BLUE));
    let uk = create_pgp_secret_packet(&keys.encryption, KeyPurpose::Encryption)?;
    card.import_key(Box::new(uk), KeyType::Decryption)?;
    println!("  {}", style("Success").color256(CLI_GREEN));

    Ok(())
}

/// Creates a PGO secret key packet from key details
fn create_pgp_secret_packet(key: &KeyInfo, kp: KeyPurpose) -> Result<UploadableKey> {
    let (pk, sp) = match kp {
        KeyPurpose::Signing => {
            // Packet Lenth is 51 octets for EdDSA Legacy Keys (which is what is most supported)
            let packet_header = PacketHeader::new_fixed(Tag::PublicKey, 51);

            let pk = PublicKey::new_with_header(
                packet_header,
                KeyVersion::V4,
                PublicKeyAlgorithm::EdDSALegacy,
                Utc::now(),
                key.expiry.map(|e| e.num_days() as u16),
                PublicParams::EdDSALegacy(EddsaLegacyPublicParams::Ed25519 {
                    key: VerifyingKey::from_bytes(
                        key.secret.get_public_bytes().first_chunk::<32>().unwrap(),
                    )?,
                }),
            )?;

            // Create SecretParams
            let sp = SecretParams::Plain(PlainSecretParams::Ed25519Legacy(
                crypto::ed25519::SecretKey::try_from_bytes(
                    *key.secret.get_private_bytes().first_chunk::<32>().unwrap(),
                    Mode::EdDSALegacy,
                )?,
            ));

            (pk, sp)
        }
        KeyPurpose::Authentication => {
            // Packet Lenth is 51 octets for EdDSA Legacy Keys (which is what is most supported)
            let packet_header = PacketHeader::new_fixed(Tag::PublicKey, 51);

            let pk = PublicKey::new_with_header(
                packet_header,
                KeyVersion::V4,
                PublicKeyAlgorithm::EdDSALegacy,
                Utc::now(),
                key.expiry.map(|e| e.num_days() as u16),
                PublicParams::EdDSALegacy(EddsaLegacyPublicParams::Ed25519 {
                    key: VerifyingKey::from_bytes(
                        key.secret.get_public_bytes().first_chunk::<32>().unwrap(),
                    )?,
                }),
            )?;

            // Create SecretParams
            let sp = SecretParams::Plain(PlainSecretParams::Ed25519Legacy(
                crypto::ed25519::SecretKey::try_from_bytes(
                    *key.secret.get_private_bytes().first_chunk::<32>().unwrap(),
                    Mode::EdDSALegacy,
                )?,
            ));

            (pk, sp)
        }
        KeyPurpose::Encryption => {
            // Packet Lenth is 56 octets for ECDH
            let packet_header = PacketHeader::new_fixed(Tag::PublicKey, 56);

            let pk = PublicKey::new_with_header(
                packet_header,
                KeyVersion::V4,
                PublicKeyAlgorithm::ECDH,
                Utc::now(),
                key.expiry.map(|e| e.num_days() as u16),
                PublicParams::ECDH(EcdhPublicParams::Curve25519 {
                    p: x25519_dalek::PublicKey::from(
                        *key.secret.get_public_bytes().first_chunk::<32>().unwrap(),
                    ),
                    hash: crypto::hash::HashAlgorithm::Sha256,
                    alg_sym: crypto::sym::SymmetricKeyAlgorithm::AES256,
                }),
            )?;

            let ss =
                StaticSecret::from(*key.secret.get_private_bytes().first_chunk::<32>().unwrap());

            // Create SecretParams
            let sp = SecretParams::Plain(PlainSecretParams::ECDH(
                crypto::ecdh::SecretKey::Curve25519(ss.into()),
            ));

            (pk, sp)
        }
        _ => bail!("Invalid Key Purpose being used to import secret key to hardware token"),
    };

    // Convert to uploadable key
    Ok(SecretKey::new(pk, sp)?.into())
}
