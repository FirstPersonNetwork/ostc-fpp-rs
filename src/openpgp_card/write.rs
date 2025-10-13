/*!
*   Handles writing of data to the OpenPGP Card
*/

use crate::{openpgp_card::KeyPurpose, setup::CommunityDIDKeys};
use affinidi_tdk::secrets_resolver::secrets::Secret;
use anyhow::Result;
use chrono::Utc;
use dialoguer::{Input, Password, theme::ColorfulTheme};
use ed25519_dalek_bip32::VerifyingKey;
use openpgp_card::{
    Card,
    ocard::{
        KeyType,
        crypto::{CardUploadableKey, EccPub},
    },
    state::Open,
};
use openpgp_card_rpgp::UploadableKey;
use pgp::{
    bytes::Bytes,
    crypto::{self, ecc_curve::ECCCurve, ed25519::Mode, public_key::PublicKeyAlgorithm},
    packet::{PacketHeader, PublicKey, SecretKey},
    types::{
        EcdhPublicParams, EcdsaPublicParams, Ed25519PublicParams, EddsaLegacyPublicParams,
        KeyVersion, PlainSecretParams, PublicParams, SecretParams, Tag,
    },
};
use secrecy::SecretString;

/// Writes keys to the card
pub fn write_keys_to_card(card: &mut Card<Open>, keys: &CommunityDIDKeys) -> Result<()> {
    // Open the card in admin mode
    let admin_pin: SecretString = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Admin PIN")
        .allow_empty_password(true)
        .interact()
        .unwrap()
        .into();

    // Check the User PIN as well

    // Create a PGP secret key packet
    let uk = create_pgp_secret_packet(&keys.signing, KeyPurpose::Signing)?;

    // Try unlocking the card with the admin PIN
    let mut open_card = card.transaction()?;
    open_card.verify_admin_pin(admin_pin)?;
    let mut card = open_card.to_admin_card(None)?;
    card.import_key(Box::new(uk), KeyType::Signing)?;

    Ok(())
}

/// Creates a PGO secret key packet from key details
fn create_pgp_secret_packet(secret: &Secret, kp: KeyPurpose) -> Result<UploadableKey> {
    // Create PublicKey
    let pk_bytes: &[u8; 32] = secret.get_public_bytes().first_chunk::<32>().unwrap();

    // Packet Lenth is 51 octets for EdDSA Legacy Keys (which is what is most supported)
    let packet_header = PacketHeader::new_fixed(Tag::PublicKey, 51);

    let pk = PublicKey::new_with_header(
        packet_header,
        KeyVersion::V4,
        PublicKeyAlgorithm::EdDSALegacy,
        Utc::now(),
        None,
        PublicParams::EdDSALegacy(EddsaLegacyPublicParams::Ed25519 {
            key: VerifyingKey::from_bytes(pk_bytes)?,
        }),
    )?;

    // Create SecretParams
    let sp = SecretParams::Plain(PlainSecretParams::Ed25519Legacy(
        crypto::ed25519::SecretKey::try_from_bytes(
            *secret.get_private_bytes().first_chunk::<32>().unwrap(),
            Mode::EdDSALegacy,
        )?,
    ));

    // Create SecretKey
    let sk = SecretKey::new(pk, sp)?;

    // Convert to uploadable key
    Ok(sk.into())
}
